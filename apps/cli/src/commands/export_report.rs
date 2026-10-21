//! The `--json` report of `mangas kindle` and `mangas export`: one object with the books
//! written, the chapters left out, the books that failed to send and the detectors' state.

use std::path::{Path, PathBuf};

use mangas_core::export::free_names_in;
use mangas_core::export::kindle::ExportStats;
use mangas_core::export::volume::VolumeOutcome;
use mangas_core::library::ExportedChapter;
use mangas_core::ml::DetectorReport;
use serde::Serialize;

use super::deliver::NOT_SENT;
use super::export_size::bytes;
use crate::output::print_json;

#[derive(Debug, Serialize)]
pub struct Report {
    pub files: Vec<WrittenFile>,
    pub skipped: Vec<Item>,
    pub failed: Vec<Item>,
    /// Books planned after one that failed: the run stopped before them.
    pub not_written: Vec<Item>,
    /// Chapter folders recorded as exported in the library, with the chapter each one is.
    pub recorded: Vec<Recorded>,
    pub detectors: DetectorReport,
}

/// A chapter folder and the library chapter it was recorded as.
#[derive(Debug, Clone, Serialize)]
pub struct Recorded {
    pub path: PathBuf,
    #[serde(flatten)]
    pub chapter: ExportedChapter,
}

#[derive(Debug, Serialize)]
pub struct WrittenFile {
    pub path: PathBuf,
    pub title: String,
    /// Chapters in the book (folders that gave no page are in `skipped` instead).
    pub chapters: usize,
    pub pages: usize,
    pub bytes: u64,
    /// Which part of the book this file is, when it was written in parts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<PartNo>,
    /// Files in its chapters left out because their format can't be decoded.
    pub skipped_files: usize,
}

#[derive(Debug, Serialize)]
pub struct PartNo {
    pub index: usize,
    pub total: usize,
}

/// A chapter folder or book, and what happened to it.
#[derive(Debug, Serialize)]
pub struct Item {
    pub path: PathBuf,
    pub reason: String,
    /// The kind of failure a script can act on: `no_space` when the disk or the Kindle is full.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'static str>,
}

/// The class of `book`'s failure from the core's message ([`Error::NoSpace`],
/// [`Error::DeviceFull`]): the message itself, or the error after `<book>`[`NOT_SENT`], must
/// start with the line, so a path that holds those words doesn't class (or hide) a failure.
///
/// [`Error::NoSpace`]: mangas_core::Error::NoSpace
/// [`Error::DeviceFull`]: mangas_core::Error::DeviceFull
pub fn error_class(book: &Path, reason: &str) -> Option<&'static str> {
    let unsent = format!("{}{NOT_SENT}", book.display());
    let error = reason.strip_prefix(&unsent).unwrap_or(reason);
    ["not enough space in ", "not enough space on "]
        .iter()
        .any(|line| error.starts_with(line))
        .then_some("no_space")
}

/// Why a chapter folder is not in its book.
pub const EMPTY_CHAPTER: &str = "no readable pages";
/// Why a planned book is not on disk after an earlier one failed.
pub const NOT_STARTED: &str = "not started: an earlier book failed";

impl Report {
    pub fn new(detectors: DetectorReport) -> Self {
        Self {
            files: Vec::new(),
            skipped: Vec::new(),
            failed: Vec::new(),
            not_written: Vec::new(),
            recorded: Vec::new(),
            detectors,
        }
    }

    /// Adds the book `stats` describes, made from the chapter folders `dirs`.
    pub fn add(&mut self, stats: &ExportStats, title: &str, dirs: &[PathBuf]) {
        let chapters = stats.exported_chapters(dirs).len();
        let total = stats.parts.len();
        let titles = stats.titled_outputs(title);
        let file = |path: &Path, pages, chapters, part: Option<PartNo>| WrittenFile {
            path: path.to_path_buf(),
            title: titles[part.as_ref().map_or(0, |p| p.index - 1)].1.clone(),
            chapters,
            pages,
            bytes: bytes(path),
            part,
            skipped_files: stats.skipped,
        };
        match total {
            0 => self
                .files
                .push(file(&stats.output, stats.pages, chapters, None)),
            _ => self
                .files
                .extend(stats.parts.iter().enumerate().map(|(i, p)| {
                    file(
                        &p.path,
                        p.pages,
                        p.chapters,
                        Some(PartNo {
                            index: i + 1,
                            total,
                        }),
                    )
                })),
        }
        self.skipped
            .extend(stats.empty_chapters.iter().map(|dir| Item {
                path: dir.clone(),
                reason: EMPTY_CHAPTER.to_string(),
                error: None,
            }));
    }

    pub fn fail(&mut self, book: &Path, reason: String) {
        self.failed.push(Item {
            path: book.to_path_buf(),
            error: error_class(book, &reason),
            reason,
        });
    }

    /// Adds the book a stopped run failed on and the ones it didn't start.
    pub fn stopped(&mut self, outcome: &VolumeOutcome) {
        if let Some(failed) = &outcome.failed {
            self.fail(&failed.output, failed.reason.clone());
        }
        self.not_written
            .extend(outcome.not_written.iter().map(|p| Item {
                path: p.output.clone(),
                reason: NOT_STARTED.to_string(),
                error: None,
            }));
    }

    /// The book at `output` failed to export: with `json`, prints the report so far (the
    /// failure in `failed`, like `export --json`), then returns the error.
    pub fn export_failed<T>(
        &mut self,
        output: &Path,
        replace: bool,
        e: mangas_core::Error,
        json: bool,
    ) -> anyhow::Result<T> {
        self.fail(&planned_book(output, replace), e.to_string());
        if json {
            self.print()?;
        }
        Err(e.into())
    }

    pub fn print(&self) -> anyhow::Result<()> {
        print_json(self)
    }
}

/// The book `export` writes for `output`: `output` itself when replacing, else its first free name
/// in the folder, as the writer picks it (a failed write leaves nothing that would move it).
pub fn planned_book(output: &Path, replace: bool) -> PathBuf {
    let dir = output.parent().filter(|d| !d.as_os_str().is_empty());
    let name = output.file_name().map(|n| n.to_string_lossy().into_owned());
    let free = match (replace, name) {
        (false, Some(name)) => free_names_in(dir.unwrap_or(Path::new(".")), &[name]).ok(),
        _ => None,
    };
    match free.as_ref().and_then(|f| f.first()) {
        Some(name) => output.with_file_name(name),
        None => output.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_book_is_named_as_the_writer_names_it() {
        let dir = tempfile::tempdir().unwrap();
        let book = dir.path().join("Book.epub");
        assert_eq!(planned_book(&book, false), book);
        std::fs::write(&book, b"older").unwrap();
        assert_eq!(planned_book(&book, false), dir.path().join("Book (2).epub"));
        assert_eq!(planned_book(&book, true), book);
    }

    #[test]
    fn a_full_disk_or_kindle_is_classed_no_space() {
        let class = |book: &str, reason: &str| error_class(Path::new(book), reason);
        let sent = "/b/A.epub was written but not sent: not enough space on Kindle at /media/k";
        assert_eq!(class("/b/A.epub", sent), Some("no_space"));
        let full = "not enough space in /out was written but not sent: x";
        assert_eq!(
            class("/out was written but not sent: x/A.epub", full),
            Some("no_space")
        );
        let denied = "/not enough space on k/A.epub was written but not sent: io: denied";
        assert_eq!(class("/not enough space on k/A.epub", denied), None);
        assert_eq!(
            class("/out/A.epub", "io: Permission denied (os error 13)"),
            None
        );
        assert_eq!(
            class("/x/A.epub", "io: /not enough space in x: denied"),
            None
        );
    }
}
