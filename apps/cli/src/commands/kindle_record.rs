//! Library bookkeeping after `mangas kindle` wrote an EPUB: chapters marked exported and the
//! book added to the export history. The EPUB exists either way, so failures only warn.

use std::path::{Path, PathBuf};

use mangas_core::export::kindle::ExportStats;
use mangas_core::library::{self, Library};

use super::deliver::{deliver, not_sent, SendArg};
use crate::{paths, Exit};

/// Records the book `stats` describes, made from the chapter folders `dirs`, in the library, then sends it when `to` says how (and to
/// which Kindle for USB). A library that won't open only warns, unless the book had to be sent:
/// failing to send it exits 2. Returns where it was sent, if it was.
pub async fn record_and_send(
    stats: &ExportStats,
    dirs: &[PathBuf],
    title: &str,
    device: &str,
    to: Option<(SendArg, Option<&Path>)>,
) -> anyhow::Result<Option<String>> {
    let unsent = |e: anyhow::Error| -> anyhow::Error {
        let (via, kindle) = to.unwrap_or((SendArg::Email, None));
        let message = not_sent(&stats.output, via, kindle, &e);
        Exit { code: 2, message }.into()
    };
    let lib = match paths::library_db().and_then(|db| Ok(library::open(&db)?)) {
        Ok(lib) => lib,
        Err(e) if to.is_some() => return Err(unsent(e)),
        Err(e) => {
            eprintln!("warning: could not open the library: {e}");
            return Ok(None);
        }
    };
    mark_exported(&lib, &stats.exported_chapters(dirs));
    let outputs = stats.titled_outputs(title);
    for (output, title) in &outputs {
        record_export(&lib, output, title, device);
    }
    let Some((via, kindle)) = to else {
        return Ok(None);
    };
    // Every part is tried; the first that fails is the error, the others were said on stderr.
    let (mut lines, mut failed) = (Vec::new(), None);
    for (output, title) in &outputs {
        let file = std::path::absolute(output)?;
        match deliver(&lib, &file, title, via, kindle).await {
            Ok(line) => lines.push(line),
            Err(e) => {
                let message = not_sent(output, via, kindle, &e);
                eprintln!("{message}");
                failed.get_or_insert(message);
            }
        }
    }
    match failed {
        Some(message) => Err(Exit { code: 2, message }.into()),
        None => Ok(Some(lines.join("\n"))),
    }
}

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
