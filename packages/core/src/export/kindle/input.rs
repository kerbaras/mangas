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
    Ok(image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .to_rgba8())
}

pub(super) fn chapter_title(dir: &Path, index: usize) -> String {
    dir.file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("Chapter {}", index + 1))
}

/// Unique identifier: hash of the title, the output path and the export time (no extra dependency).
pub(super) fn uuid_like(title: &str, output: &Path) -> String {
    use sha2::{Digest, Sha256};
    let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let hash = Sha256::digest(format!("{title}:{}:{now}", output.display()));
    let h: Vec<String> = hash.iter().take(16).map(|b| format!("{b:02x}")).collect();
    let s = h.concat();
    format!(
        "{}-{}-{}-{}-{}",
        &s[0..8],
        &s[8..12],
        &s[12..16],
        &s[16..20],
        &s[20..32]
    )
}
