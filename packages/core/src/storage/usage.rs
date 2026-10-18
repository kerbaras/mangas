use std::path::Path;

use serde::Serialize;

use super::siblings::chapter_files;
use super::tree::dir_size;
use super::Folders;
use crate::domain::Result;
use crate::library::{downloaded_chapters, DownloadedChapter, Library};

/// Disk space one followed title's downloaded chapters take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TitleUsage {
    pub source: String,
    pub manga_id: String,
    pub title: String,
    pub chapters: usize,
    pub bytes: u64,
}

/// Disk space per app folder, in bytes, plus the per-title breakdown of `chapters`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Usage {
    pub data_dir: String,
    pub chapters: u64,
    pub models: u64,
    pub covers: u64,
    /// The library database with its WAL files.
    pub library: u64,
    pub logs: u64,
    pub titles: Vec<TitleUsage>,
}

/// Sums chapter sizes per title, largest first (then by title, for a stable order).
pub fn group_by_title(chapters: &[(DownloadedChapter, u64)]) -> Vec<TitleUsage> {
    let mut titles: Vec<TitleUsage> = Vec::new();
    for (c, bytes) in chapters {
        match titles
            .iter_mut()
            .find(|t| t.source == c.source && t.manga_id == c.manga_id)
        {
            Some(t) => {
                t.chapters += 1;
                t.bytes += bytes;
            }
            None => titles.push(TitleUsage {
                source: c.source.clone(),
                manga_id: c.manga_id.clone(),
                title: c.manga_title.clone(),
                chapters: 1,
                bytes: *bytes,
            }),
        }
    }
    titles.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.title.cmp(&b.title)));
    titles
}

/// Measures every app folder and each title's recorded chapter folders with the archives
/// saved beside them.
pub fn usage(lib: &Library, folders: &Folders) -> Result<Usage> {
    let chapters: Vec<_> = downloaded_chapters(lib, None)?
        .into_iter()
        .map(|c| {
            let dir = Path::new(&c.path);
            let beside: u64 = chapter_files(dir).iter().map(|(_, b)| b).sum();
            let bytes = dir_size(dir) + beside;
            (c, bytes)
        })
        .collect();
    Ok(Usage {
        data_dir: folders.data_dir.to_string_lossy().into_owned(),
        chapters: dir_size(&folders.chapters()),
        models: dir_size(&folders.models()),
        covers: dir_size(&folders.covers),
        library: database_size(&folders.database),
        logs: dir_size(&folders.logs),
        titles: group_by_title(&chapters),
    })
}

fn database_size(db: &Path) -> u64 {
    ["", "-wal", "-shm"]
        .iter()
        .filter_map(|suffix| {
            let mut name = db.as_os_str().to_owned();
            name.push(suffix);
            std::fs::metadata(name).ok().map(|m| m.len())
        })
        .sum()
}
