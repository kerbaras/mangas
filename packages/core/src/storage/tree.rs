//! Filesystem walks over a folder tree. A link to a file counts as that file; links to
//! folders are not followed.

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::domain::Result;

/// A regular file under a root: its path relative to the root and its size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileEntry {
    pub rel: PathBuf,
    pub bytes: u64,
}

/// Total size of the regular files under `path`; 0 when it doesn't exist. Unreadable entries
/// are skipped: usage is informational and shouldn't fail on one bad file.
pub fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    walk(path, Path::new(""), &mut |e| total += e.bytes, false).ok();
    total
}

/// Every regular file under `root`, sorted by relative path; empty when `root` doesn't exist.
/// Unlike [`dir_size`], an unreadable entry is an error, because a copy must not miss files.
pub(crate) fn list_files(root: &Path) -> Result<Vec<FileEntry>> {
    let mut files = Vec::new();
    if root.exists() {
        walk(root, Path::new(""), &mut |e| files.push(e), true)?;
    }
    files.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(files)
}

fn walk(
    dir: &Path,
    rel: &Path,
    visit: &mut dyn FnMut(FileEntry),
    strict: bool,
) -> std::io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if strict => return Err(e),
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) if strict => return Err(e),
            Err(_) => continue,
        };
        let kind = match entry.file_type() {
            Ok(kind) => kind,
            Err(e) if strict => return Err(e),
            Err(_) => continue,
        };
        let rel = rel.join(entry.file_name());
        if kind.is_dir() {
            walk(&entry.path(), &rel, visit, strict)?;
        } else if kind.is_file() {
            match entry.metadata() {
                Ok(meta) => visit(FileEntry {
                    rel,
                    bytes: meta.len(),
                }),
                Err(e) if strict => return Err(e),
                Err(_) => {}
            }
        } else if kind.is_symlink() {
            match fs::metadata(entry.path()) {
                Ok(meta) if meta.is_file() => visit(FileEntry {
                    rel,
                    bytes: meta.len(),
                }),
                Ok(_) if strict => {
                    return Err(std::io::Error::other(format!(
                        "{} is a link to a folder; replace it with the folder itself first",
                        entry.path().display()
                    )))
                }
                Err(e) if strict => return Err(e),
                _ => {}
            }
        }
    }
    Ok(())
}

/// Removes the files directly and recursively under `dir`, keeping `dir` itself and the
/// top-level files named in `keep`. Returns how many files and bytes went.
pub(crate) fn empty_dir(dir: &Path, keep: &[&str]) -> Result<(usize, u64)> {
    let kept = |rel: &Path| keep.iter().any(|k| rel == Path::new(k));
    let files: Vec<_> = list_files(dir)?
        .into_iter()
        .filter(|f| !kept(&f.rel))
        .collect();
    for entry in fs::read_dir(dir).into_iter().flatten() {
        let entry = entry?;
        if kept(Path::new(&entry.file_name())) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() && !path.is_symlink() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok((files.len(), files.iter().map(|f| f.bytes).sum()))
}

/// `dir` and those of its ancestors that don't exist yet, deepest first. Only up to a `..`:
/// once the folders before it exist, the rest may name a folder that was there all along.
pub(crate) fn missing_dirs(dir: &Path) -> Vec<PathBuf> {
    let plain: PathBuf = (dir.components())
        .take_while(|c| *c != Component::ParentDir)
        .collect();
    plain
        .ancestors()
        .take_while(|d| !d.as_os_str().is_empty() && fs::symlink_metadata(d).is_err())
        .map(Path::to_path_buf)
        .collect()
}

/// Removes the folders in `dirs` (deepest first) that are empty; a folder something was put in
/// since stays, and so do its ancestors.
pub(crate) fn remove_empty(dirs: &[PathBuf]) {
    for dir in dirs {
        if fs::remove_dir(dir).is_err() && dir.exists() {
            return;
        }
    }
}
