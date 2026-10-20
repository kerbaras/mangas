//! Library lookups by chapter folder for `mangas kindle`.

use std::path::{Path, PathBuf};

use mangas_core::library::{self, Library};

/// Season-aware TOC entries; empty (folder names) unless the library knows every folder.
pub(super) fn titles_in(lib: &Library, chapters: &[PathBuf]) -> anyhow::Result<Vec<String>> {
    let spellings = chapters
        .iter()
        .map(|d| spellings(lib, &[d.as_path()]))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let dirs: Vec<Vec<&Path>> = spellings
        .iter()
        .map(|s| s.iter().map(PathBuf::as_path).collect())
        .collect();
    Ok(library::chapter_titles(lib, &dirs)?)
}

/// Each folder as given, made absolute and resolved (symlinks), plus the stored download folder
/// it is a copy of: the library may store any of them.
pub(super) fn spellings(lib: &Library, dirs: &[&Path]) -> anyhow::Result<Vec<PathBuf>> {
    let mut spellings = Vec::new();
    for dir in dirs {
        let own: Vec<PathBuf> = [
            Some(dir.to_path_buf()),
            std::path::absolute(dir).ok(),
            dir.canonicalize().ok(),
        ]
        .into_iter()
        .flatten()
        .collect();
        let refs: Vec<&Path> = own.iter().map(PathBuf::as_path).collect();
        let original = library::stored_original(lib, &refs)?;
        spellings.extend(own);
        spellings.extend(original);
    }
    spellings.sort();
    spellings.dedup();
    Ok(spellings)
}
