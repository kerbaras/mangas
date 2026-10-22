//! The `--json` report of `mangas kindle` and `mangas export`: one object with the books
//! written, the chapters left out, the books that failed to send and the detectors' state.

use std::path::{Path, PathBuf};

use mangas_core::export::kindle::ExportStats;
use mangas_core::export::volume::VolumeOutcome;
use mangas_core::library::ExportedChapter;
use mangas_core::ml::DetectorReport;
use serde::Serialize;

use super::deliver::Unsent;
use super::export_plan::{planned_book, PlannedBook};
use super::export_size::bytes;
use crate::output::print_json;

#[derive(Debug, Serialize)]
pub struct Report {
    /// The plan made before writing (`mangas export`): each book's estimate against its budget.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub plan: Vec<PlannedBook>,
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
    /// JPEG quality its pages were encoded at.
    pub jpeg_quality: u8,
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
    /// What a script can act on: `no_space` (disk or Kindle full), `usb_refused` / `usb_warn`
    /// (see `--force`), `kindle_over_mtp` (no USB copy), `email_refused` (EPUB and PDF only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'static str>,
}

/// The class of a failure from the core's message ([`Error::NoSpace`], [`Error::DeviceFull`]),
/// which starts with the line; a path later in the message can't class it.
///
/// [`Error::NoSpace`]: mangas_core::Error::NoSpace
/// [`Error::DeviceFull`]: mangas_core::Error::DeviceFull
pub fn error_class(error: &str) -> Option<&'static str> {
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
            plan: Vec::new(),
            files: Vec::new(),
            skipped: Vec::new(),
            failed: Vec::new(),
            not_written: Vec::new(),
            recorded: Vec::new(),
            detectors,
        }
    }

    pub fn with_plan(self, plan: Vec<PlannedBook>) -> Self {
        Self { plan, ..self }
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
            jpeg_quality: stats.jpeg_quality,
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
            error: error_class(&reason),
            reason,
        });
    }

    /// Adds a written book that wasn't sent, with its delivery error's class.
    pub fn unsent(&mut self, book: &Path, unsent: &Unsent) {
        self.failed.push(Item {
            path: book.to_path_buf(),
            reason: unsent.message.clone(),
            error: unsent.class,
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

#[cfg(test)]
#[path = "export_report_tests.rs"]
mod tests;
