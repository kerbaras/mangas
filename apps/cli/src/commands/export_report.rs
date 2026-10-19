//! The `--json` report of `mangas kindle` and `mangas export`: one object with the books
//! written, the chapters left out, the books that failed to send and the detectors' state.

use std::path::{Path, PathBuf};

use mangas_core::export::kindle::ExportStats;
use mangas_core::ml::DetectorReport;
use serde::Serialize;

use crate::output::print_json;

#[derive(Debug, Serialize)]
pub struct Report {
    pub files: Vec<WrittenFile>,
    pub skipped: Vec<Item>,
    pub failed: Vec<Item>,
    pub detectors: DetectorReport,
}

#[derive(Debug, Serialize)]
pub struct WrittenFile {
    pub path: PathBuf,
    pub title: String,
    /// Chapters in the book (folders that gave no page are in `skipped` instead).
    pub chapters: usize,
    pub pages: usize,
    /// Files in its chapters left out because their format can't be decoded.
    pub skipped_files: usize,
}

/// A chapter folder or book, and what happened to it.
#[derive(Debug, Serialize)]
pub struct Item {
    pub path: PathBuf,
    pub reason: String,
}

/// Why a chapter folder is not in its book.
pub const EMPTY_CHAPTER: &str = "no readable pages";

impl Report {
    pub fn new(detectors: DetectorReport) -> Self {
        Self {
            files: Vec::new(),
            skipped: Vec::new(),
            failed: Vec::new(),
            detectors,
        }
    }

    /// Adds the book `stats` describes, made from the chapter folders `dirs`.
    pub fn add(&mut self, stats: &ExportStats, title: &str, dirs: &[PathBuf]) {
        self.files.push(WrittenFile {
            path: stats.output.clone(),
            title: title.to_string(),
            chapters: stats.exported_chapters(dirs).len(),
            pages: stats.pages,
            skipped_files: stats.skipped,
        });
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

    pub fn print(&self) -> anyhow::Result<()> {
        print_json(self)
    }
}
