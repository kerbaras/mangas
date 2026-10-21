//! Library bookkeeping after `mangas kindle` wrote an EPUB: chapters marked exported and the
//! book added to the export history. The EPUB exists either way, so failures only warn.

use std::path::{Path, PathBuf};

use mangas_core::export::kindle::ExportStats;
use mangas_core::library::{self, Library};

use super::deliver::{deliver, not_sent, SendArg, Unsent};
use super::export_report::Recorded;
use super::kindle_titles::spellings;
use crate::output::outln;
use crate::{paths, Exit};

/// Where each file went, and the files that couldn't be sent with why.
#[derive(Default)]
pub struct Delivery {
    pub recorded: Vec<Recorded>,
    pub sent: Vec<String>,
    pub failed: Vec<(PathBuf, Unsent)>,
}

impl Delivery {
    /// Exit 2 when a file wasn't sent: naming it, or counting them after saying each on stderr.
    pub fn result(&self) -> anyhow::Result<()> {
        let message = match self.failed.as_slice() {
            [] => return Ok(()),
            [(_, unsent)] => unsent.message.clone(),
            many => {
                many.iter().for_each(|(_, u)| eprintln!("{}", u.message));
                format!("{} parts were written but not sent", many.len())
            }
        };
        Err(Exit { code: 2, message }.into())
    }

    /// Says each send on stdout, or on stderr when stdout carries the JSON report.
    pub fn print_sent(&self, json: bool) -> std::io::Result<()> {
        for line in &self.sent {
            match json {
                true => eprintln!("{line}"),
                false => outln!("{line}")?,
            }
        }
        Ok(())
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
                failed,
                ..Delivery::default()
            };
        }
    };
    let mut done = Delivery {
        recorded: mark_exported(&lib, stats, dirs),
        ..Delivery::default()
    };
    for (output, title) in &outputs {
        record_export(&lib, output, title, device);
    }
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

/// Library chapters among the folders of `dirs` in the book `stats` describes count as read
/// from now on, and are returned with their folder;
/// folders from elsewhere are ignored. Each folder is matched under every spelling the library
/// may store and as a moved or copied download ([`spellings`], the match the book's title uses).
pub fn mark_exported(lib: &Library, stats: &ExportStats, dirs: &[PathBuf]) -> Vec<Recorded> {
    let marked = |dir: &&Path| -> anyhow::Result<Vec<Recorded>> {
        let spellings = spellings(lib, &[dir])?;
        let refs: Vec<&Path> = spellings.iter().map(PathBuf::as_path).collect();
        let chapters = library::mark_exported_folder(lib, &refs)?;
        Ok(chapters
            .into_iter()
            .map(|chapter| Recorded {
                path: dir.to_path_buf(),
                chapter,
            })
            .collect())
    };
    stats
        .exported_chapters(dirs)
        .iter()
        .flat_map(|dir| {
            marked(dir).unwrap_or_else(|e| {
                eprintln!("warning: could not mark {} as exported: {e}", dir.display());
                Vec::new()
            })
        })
        .collect()
}

/// Adds the EPUB at `output` (made absolute) to the export history.
pub fn record_export(lib: &Library, output: &Path, title: &str, device: &str) {
    let file = std::path::absolute(output).unwrap_or_else(|_| output.to_path_buf());
    if let Err(e) = library::record_export(lib, &file, title, device) {
        eprintln!("warning: could not record the export: {e}");
    }
}
