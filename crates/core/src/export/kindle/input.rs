//! Reading chapter directories: image listing, decoding and naming helpers.

use std::path::{Path, PathBuf};

use image::RgbaImage;

use crate::domain::Result;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// Image files of a chapter directory in name order.
pub fn chapter_images(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    files.sort();
    Ok(files)
}

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
