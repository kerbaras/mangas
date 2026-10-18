//! The on-disk side of the cover cache: one file per manga, written atomically.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use super::evict::{to_evict, Entry};
use super::key::{file_stem, image_extension, manga_prefix, EXTENSIONS};
use crate::domain::{Error, Result};

/// How much the cache may hold before the least recently used covers are deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_files: usize,
    pub max_bytes: u64,
}

impl Default for Limits {
    /// Room for a large library of full-size covers.
    fn default() -> Self {
        Self {
            max_files: 2000,
            max_bytes: 200 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CoverCache {
    dir: PathBuf,
    limits: Limits,
}

impl CoverCache {
    pub fn new(dir: impl Into<PathBuf>, limits: Limits) -> Self {
        Self {
            dir: dir.into(),
            limits,
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The cached cover for this manga and URL, marked as just used.
    pub fn lookup(&self, source: &str, manga_id: &str, url: &str) -> Option<PathBuf> {
        let stem = file_stem(source, manga_id, url);
        let path = EXTENSIONS
            .iter()
            .map(|ext| self.dir.join(format!("{stem}.{ext}")))
            .find(|p| p.is_file())?;
        touch(&path);
        Some(path)
    }

    /// Writes an image body as this manga's cover, replacing older covers of the same manga,
    /// then trims the cache to its limits. Bodies that are not images are refused.
    pub fn store(&self, source: &str, manga_id: &str, url: &str, bytes: &[u8]) -> Result<PathBuf> {
        let ext = image_extension(bytes)
            .ok_or_else(|| Error::Other(format!("cover {url}: not an image")))?;
        fs::create_dir_all(&self.dir)?;
        let name = format!("{}.{ext}", file_stem(source, manga_id, url));
        let path = self.dir.join(&name);
        let tmp = self.dir.join(format!(".{name}.{}.tmp", unique()));
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &path)?;
        self.remove_older(&manga_prefix(source, manga_id), &name);
        self.trim();
        Ok(path)
    }

    fn remove_older(&self, prefix: &str, keep: &str) {
        for name in self.names() {
            if name.starts_with(prefix) && name != keep {
                let _ = fs::remove_file(self.dir.join(name));
            }
        }
    }

    fn trim(&self) {
        let entries = self.entries();
        for name in to_evict(entries, self.limits.max_files, self.limits.max_bytes) {
            let _ = fs::remove_file(self.dir.join(name));
        }
    }

    /// Cover files (temporary files of writes in progress are skipped).
    fn names(&self) -> Vec<String> {
        let Ok(dir) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        dir.filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|n| !n.starts_with('.'))
            .collect()
    }

    fn entries(&self) -> Vec<Entry> {
        self.names()
            .into_iter()
            .filter_map(|name| {
                let meta = fs::metadata(self.dir.join(&name)).ok()?;
                let used = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                Some(Entry {
                    name,
                    size: meta.len(),
                    used,
                })
            })
            .collect()
    }
}

/// Marks a cover as used now (its mtime orders eviction). Best effort.
fn touch(path: &Path) {
    let _ = fs::File::options()
        .write(true)
        .open(path)
        .and_then(|f| f.set_modified(SystemTime::now()));
}

/// Distinguishes concurrent writes of the same cover within and across processes.
fn unique() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
