//! Reading chapter directories: image listing, decoding and naming helpers.

use std::path::{Path, PathBuf};

use image::RgbaImage;

use crate::domain::Result;
use crate::images::has_page_extension;

/// Files of a chapter directory in name order: the pages to export and the files skipped
/// because their format is not decodable. Hidden files and subdirectories are ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChapterFiles {
    pub pages: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
}

pub fn chapter_files(dir: &Path) -> Result<ChapterFiles> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|p| p.is_file() && !is_hidden(p))
        .collect();
    files.sort();
    let (pages, skipped) = files.into_iter().partition(|p| has_page_extension(p));
    Ok(ChapterFiles { pages, skipped })
}

/// Image files of a chapter directory in name order.
pub fn chapter_images(dir: &Path) -> Result<Vec<PathBuf>> {
    Ok(chapter_files(dir)?.pages)
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}

/// Decodes by content, not extension; an animated GIF yields its first frame.
pub fn decode(path: &Path) -> Result<RgbaImage> {
    if let Some(img) = super::png_rgba::decode(path) {
        return Ok(img);
    }
    Ok(image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .into_rgba8())
}

pub(super) fn chapter_title(dir: &Path, index: usize) -> String {
    dir.file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("Chapter {}", index + 1))
}

/// The book's `dc:identifier`: a UUIDv5 of `key`, so exporting the same chapters again gives
/// the same id and Kindle and calibre treat the new file as the same book.
pub(super) fn book_id(key: &str) -> String {
    uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_URL,
        format!("mangas:{key}").as_bytes(),
    )
    .to_string()
}
