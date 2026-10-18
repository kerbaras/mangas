//! Library lookups by chapter folder for `mangas kindle`.

use std::path::{Path, PathBuf};

use mangas_core::library::{self, Library};

/// Season-aware TOC entries; empty (folder names) unless the library knows every folder.
pub(super) fn titles_in(lib: &Library, chapters: &[PathBuf]) -> anyhow::Result<Vec<String>> {
    let spellings: Vec<Vec<PathBuf>> = chapters.iter().map(|d| spellings(&[d.as_path()])).collect();
    let dirs: Vec<Vec<&Path>> = spellings
        .iter()
        .map(|s| s.iter().map(PathBuf::as_path).collect())
        .collect();
    Ok(library::chapter_titles(lib, &dirs)?)
}

/// Each folder as given, made absolute and resolved (symlinks): the library may store any of them.
pub(super) fn spellings(dirs: &[&Path]) -> Vec<PathBuf> {
    let mut spellings: Vec<PathBuf> = dirs
        .iter()
        .flat_map(|d| {
            [
                Some(d.to_path_buf()),
                std::path::absolute(d).ok(),
                d.canonicalize().ok(),
            ]
        })
        .flatten()
        .collect();
    spellings.sort();
    spellings.dedup();
    spellings
}
