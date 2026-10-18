//! What an export is asked to do and what it reports back.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::Device;
use crate::export::epub::Series;
use crate::smartcut::Options;

/// Book-level metadata; the default is a plain chapter bundle titled by `ExportRequest::title`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BookMeta {
    /// `dc:creator`s; "mangas" when empty.
    pub authors: Vec<String>,
    /// `dc:language`; "en" when unknown.
    pub language: Option<String>,
    pub series: Option<Series>,
    /// Series cover image; the first page is the cover when absent or unreadable.
    pub cover: Option<PathBuf>,
    /// TOC label per `chapter_dirs` entry; missing ones use the folder name.
    pub chapter_titles: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExportRequest {
    pub chapter_dirs: Vec<PathBuf>,
    pub device: Device,
    pub cut: Options,
    pub title: String,
    pub output: PathBuf,
    pub meta: BookMeta,
}

/// Progress events, in order: one `Chapter` per directory, `Pages` as pages are finished, `Writing`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Progress {
    Chapter {
        index: usize,
        total: usize,
        path: PathBuf,
    },
    Pages {
        done: usize,
    },
    Writing {
        pages: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportStats {
    pub pages: usize,
    pub strips: usize,
    pub source_rows: usize,
    pub pagination_ms: u128,
    pub filters_ms: u128,
    /// Files left out because their format cannot be decoded (each one is logged).
    #[serde(default)]
    pub skipped: usize,
    /// Chapter folders that gave no page (nothing decodable in them): not part of the book.
    #[serde(default)]
    pub empty_chapters: Vec<PathBuf>,
    /// File actually written (differs from the request when the name was already taken).
    pub output: PathBuf,
}

impl ExportStats {
    /// The requested chapter folders that made it into the book.
    pub fn exported_chapters<'a>(&self, dirs: &'a [PathBuf]) -> Vec<&'a Path> {
        dirs.iter()
            .filter(|d| !self.empty_chapters.contains(d))
            .map(PathBuf::as_path)
            .collect()
    }
}
