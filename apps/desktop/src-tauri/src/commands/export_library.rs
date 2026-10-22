//! What a chapter-folder export reads from the library: labels, TOC entries and creators.

use std::path::{Path, PathBuf};

use mangas_core::export::Labels;
use mangas_core::library;

use super::{open_library, to_ipc};

/// Series/chapter/volume labels from the library; without it the file is just unlabelled.
pub(super) fn labels_of(dirs: &[PathBuf]) -> Labels {
    let refs: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
    open_library()
        .and_then(|lib| library::export_labels(&lib, &refs, dirs.len()).map_err(to_ipc))
        .unwrap_or_else(|e| {
            tracing::warn!("could not read chapter labels: {e}");
            Labels::default()
        })
}

/// Season-aware TOC entries from the library; empty (folder names) when it doesn't know them.
pub(super) fn titles_of(dirs: &[PathBuf]) -> Vec<String> {
    let spellings: Vec<Vec<&Path>> = dirs.iter().map(|d| vec![d.as_path()]).collect();
    open_library()
        .and_then(|lib| library::chapter_titles(&lib, &spellings).map_err(to_ipc))
        .unwrap_or_else(|e| {
            tracing::warn!("could not read chapter titles: {e}");
            Vec::new()
        })
}

/// Who the book credits, from the series of the folders; nobody when the library names no one.
pub(super) fn creators_of(dirs: &[PathBuf]) -> Vec<String> {
    let spellings: Vec<Vec<&Path>> = dirs.iter().map(|d| vec![d.as_path()]).collect();
    open_library()
        .and_then(|lib| library::export_creators(&lib, &spellings).map_err(to_ipc))
        .unwrap_or_else(|e| {
            tracing::warn!("could not read the book's creators: {e}");
            Vec::new()
        })
}
