//! Chapter downloads: fetch page images from a [`crate::sources::Source`] into the chapter
//! directory, with bounded concurrency and resume. Implemented in the "sources + downloads" PR.

use std::path::Path;

use crate::domain::{ChapterId, Error, Result};

/// Downloads one chapter into `dest` and returns the number of pages written.
pub async fn download_chapter(_chapter: &ChapterId, _dest: &Path) -> Result<usize> {
    Err(Error::NotImplemented("download"))
}
