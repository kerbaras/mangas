//! A book `mangas kindle`/`mangas export` wrote, as its `--json` report lists it.

use std::path::PathBuf;

use serde::Serialize;

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
