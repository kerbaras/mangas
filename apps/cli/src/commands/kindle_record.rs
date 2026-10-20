//! Library bookkeeping after `mangas kindle` wrote an EPUB: chapters marked exported and the
//! book added to the export history. The EPUB exists either way, so failures only warn.

use std::path::{Path, PathBuf};

use mangas_core::export::kindle::ExportStats;
use mangas_core::library::{self, Library};

use super::deliver::{deliver, not_sent, SendArg};
use crate::{paths, Exit};

/// Where each file went, and the files that couldn't be sent with why.
#[derive(Default)]
pub struct Delivery {
    pub sent: Vec<String>,
    pub failed: Vec<(PathBuf, String)>,
}

impl Delivery {
    /// Exit 2 when a file wasn't sent: naming it, or counting them after saying each on stderr.
    pub fn result(&self) -> anyhow::Result<()> {
        let message = match self.failed.as_slice() {
            [] => return Ok(()),
            [(_, message)] => message.clone(),
            many => {
                many.iter().for_each(|(_, m)| eprintln!("{m}"));
                format!("{} parts were written but not sent", many.len())
            }
        };
        Err(Exit { code: 2, message }.into())
    }
}

/// Records the book `stats` describes (every part of it), made from the chapter folders `dirs`,
/// in the library, then sends each file when `to` says how (and to which Kindle for USB). A
/// library that won't open only warns, unless the book had to be sent: then no file is sent.
pub async fn record_and_send(
    stats: &ExportStats,
    dirs: &[PathBuf],
    title: &str,
    device: &str,
    to: Option<(SendArg, Option<&Path>)>,
) -> Delivery {
    let outputs = stats.titled_outputs(title);
    let lib = match paths::library_db().and_then(|db| Ok(library::open(&db)?)) {
        Ok(lib) => lib,
        Err(e) => {
            let Some((via, kindle)) = to else {
                eprintln!("warning: could not open the library: {e}");
                return Delivery::default();
            };
            let unsent =
                |(out, _): &(PathBuf, String)| (out.clone(), not_sent(out, via, kindle, &e));
            let failed = outputs.iter().map(unsent).collect();
            return Delivery {
                sent: Vec::new(),
                failed,
            };
        }
    };
    mark_exported(&lib, &stats.exported_chapters(dirs));
    for (output, title) in &outputs {
        record_export(&lib, output, title, device);
    }
    let mut done = Delivery::default();
    let Some((via, kindle)) = to else {
        return done;
    };
    for (output, title) in &outputs {
        let file = std::path::absolute(output).unwrap_or_else(|_| output.clone());
        match deliver(&lib, &file, title, via, kindle).await {
            Ok(line) => done.sent.push(line),
            Err(e) => done
                .failed
                .push((output.clone(), not_sent(output, via, kindle, &e))),
        }
    }
    done
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
