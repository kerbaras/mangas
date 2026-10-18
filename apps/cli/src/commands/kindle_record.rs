//! Library bookkeeping after `mangas kindle` wrote an EPUB: chapters marked exported and the
//! book added to the export history. The EPUB exists either way, so failures only warn.

use std::path::{Path, PathBuf};

use mangas_core::library::{self, Library};

/// Library chapters among `dirs` count as read from now on; folders from elsewhere are ignored.
/// Each folder is matched as made absolute and as resolved (symlinks), since the library may
/// store either spelling.
pub fn mark_exported(lib: &Library, dirs: &[&Path]) {
    let mut spellings: Vec<PathBuf> = dirs
        .iter()
        .flat_map(|d| [std::path::absolute(d).ok(), d.canonicalize().ok()])
        .flatten()
        .collect();
    spellings.sort();
    spellings.dedup();
    let dirs: Vec<&Path> = spellings.iter().map(PathBuf::as_path).collect();
    if let Err(e) = library::mark_exported(lib, &dirs) {
        eprintln!("warning: could not mark the chapters as exported: {e}");
    }
}

/// Adds the EPUB at `output` (made absolute) to the export history.
pub fn record_export(lib: &Library, output: &Path, title: &str, device: &str) {
    let file = std::path::absolute(output).unwrap_or_else(|_| output.to_path_buf());
    if let Err(e) = library::record_export(lib, &file, title, device) {
        eprintln!("warning: could not record the export: {e}");
    }
}
