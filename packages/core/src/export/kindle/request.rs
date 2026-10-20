//! What an export is asked to do and what it reports back.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::parts::Oversized;
use super::Device;
use crate::export::epub::Series;
use crate::export::{Format, Labels, PageSettings};
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
    /// What the book is (source, manga, chapters), hashed into its identifier; the title and
    /// chapter folders when absent.
    pub identity: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExportRequest {
    pub chapter_dirs: Vec<PathBuf>,
    pub device: Device,
    pub format: Format,
    pub cut: Options,
    /// Page height, margin trim and filter tweaks; the default leaves the profile as is.
    pub page: PageSettings,
    pub title: String,
    /// The user typed `title`; it then beats the library's chapter title in ComicInfo.
    pub title_explicit: bool,
    pub output: PathBuf,
    /// Write over a file already at `output` instead of next to it as ` (2)`, ....
    pub replace: bool,
    pub meta: BookMeta,
    /// Series, chapter and volume labels from the library (CBZ writes them to ComicInfo).
    pub labels: Labels,
    /// Write numbered parts of at most this many bytes when the book would be bigger.
    pub split: Option<u64>,
}

impl ExportRequest {
    /// The same export with `page` tweaks.
    pub fn with_page(self, page: PageSettings) -> ExportRequest {
        ExportRequest { page, ..self }
    }

    /// The same export, replacing an existing file at the output path when `replace`.
    pub fn with_replace(self, replace: bool) -> ExportRequest {
        ExportRequest { replace, ..self }
    }

    /// The same export in parts of at most `split` bytes (see [`ExportRequest::split`]).
    pub fn with_split(self, split: Option<u64>) -> ExportRequest {
        ExportRequest { split, ..self }
    }

    /// The same export written as `format`, the output's extension changed to match.
    pub fn with_format(self, format: Format) -> ExportRequest {
        ExportRequest {
            split: None,
            output: self.output.with_extension(format.id()),
            format,
            ..self
        }
    }
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
    /// File actually written (differs from the request when the name was already taken); the
    /// first part when the book was split.
    pub output: PathBuf,
    /// Every part in order when the book was split into several files, else empty.
    #[serde(default)]
    pub parts: Vec<Part>,
    /// Why detectors were asked for but not used (set by the caller that chose them).
    #[serde(default)]
    pub detectors_error: Option<String>,
    /// Files written over the Send-to-Kindle email limit, measured on disk.
    #[serde(default)]
    pub oversized: Vec<Oversized>,
}

/// One file of a book written in parts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    pub path: PathBuf,
    pub pages: usize,
    /// Chapters with at least one page in this part.
    #[serde(default)]
    pub chapters: usize,
}

impl ExportStats {
    /// The files written: the parts, else the one book.
    pub fn outputs(&self) -> Vec<PathBuf> {
        match self.parts.is_empty() {
            true => vec![self.output.clone()],
            false => self.parts.iter().map(|p| p.path.clone()).collect(),
        }
    }

    /// Each file written with the title it goes by: "Title (part 2 of 3)" for a part, else
    /// `title`. What the history records and an email's subject.
    pub fn titled_outputs(&self, title: &str) -> Vec<(PathBuf, String)> {
        let total = self.parts.len();
        match total {
            0 => vec![(self.output.clone(), title.to_string())],
            _ => (self.parts.iter().enumerate())
                .map(|(i, p)| {
                    (
                        p.path.clone(),
                        super::parts::part_title(title, i + 1, total),
                    )
                })
                .collect(),
        }
    }

    /// The requested chapter folders that made it into the book.
    pub fn exported_chapters<'a>(&self, dirs: &'a [PathBuf]) -> Vec<&'a Path> {
        dirs.iter()
            .filter(|d| !self.empty_chapters.contains(d))
            .map(PathBuf::as_path)
            .collect()
    }
}
