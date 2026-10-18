//! Files a chapter leaves next to its folder, such as `<chapter>.cbz`.

use std::fs;
use std::path::{Path, PathBuf};

/// Regular files beside `dir` whose stem is the chapter folder's name (`2.5` owns `2.5.cbz`,
/// `2` doesn't), with their sizes. Links are skipped: they aren't the chapter's own data.
pub(crate) fn chapter_files(dir: &Path) -> Vec<(PathBuf, u64)> {
    let (Some(parent), Some(name)) = (dir.parent(), dir.file_name()) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).ok()?;
            let owned =
                meta.is_file() && path.extension().is_some() && path.file_stem() == Some(name);
            owned.then_some((path, meta.len()))
        })
        .collect()
}
