//! The `--json` report of `mangas kindle` and `mangas export`: one object with the books
//! written, the chapters left out, the books that failed to send and the detectors' state.

use std::path::{Path, PathBuf};

use mangas_core::export::kindle::ExportStats;
use mangas_core::export::volume::VolumeOutcome;
use mangas_core::ml::DetectorReport;
use serde::Serialize;

use super::export_size::bytes;
use crate::output::print_json;

#[derive(Debug, Serialize)]
pub struct Report {
    pub files: Vec<WrittenFile>,
    pub skipped: Vec<Item>,
    pub failed: Vec<Item>,
    /// Books planned after one that failed: the run stopped before them.
    pub not_written: Vec<Item>,
    pub detectors: DetectorReport,
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
            }));
    }

    pub fn fail(&mut self, book: &Path, reason: String) {
        self.failed.push(Item {
            path: book.to_path_buf(),
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
            }));
    }

    pub fn print(&self) -> anyhow::Result<()> {
        print_json(self)
    }
}
