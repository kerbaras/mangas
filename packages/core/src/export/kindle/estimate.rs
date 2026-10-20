//! How big an export will be, before it runs: a few pages are encoded for real and their
//! average size is scaled to the page count read from the image headers. Books of regular pages
//! sample pages spread over every chapter, since scans vary a lot between chapters; webtoon
//! strips sample the first pages, as a strip only cuts in order.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::input::{chapter_files, ChapterFiles};
use super::run::{Run, RunSpec};
use super::Device;
use crate::domain::Result;
use crate::export::PageSettings;
use crate::smartcut::{is_long_strip, Options};

/// Pages encoded to measure the average page.
const SAMPLE: usize = 6;
/// Bytes a page costs in the book besides its JPEG.
const PAGE_OVERHEAD: u64 = 2_048;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Estimate {
    pub pages: usize,
    pub bytes: u64,
}

/// The estimated size of the book `chapter_dirs` make on `device` with `cut` and `page`.
pub fn estimate(
    chapter_dirs: &[PathBuf],
    device: Device,
    cut: Options,
    page: PageSettings,
) -> Result<Estimate> {
    let chapters: Vec<(&Path, Vec<PathBuf>)> = chapter_dirs
        .iter()
        .map(|d| {
            (
                d.as_path(),
                chapter_files(d).map(|f| f.pages).unwrap_or_default(),
            )
        })
        .collect();
    let counts: Vec<(usize, bool)> = chapters
        .iter()
        .map(|(_, files)| page_count(files, &device, &page))
        .collect();
    let pages: usize = counts.iter().map(|c| c.0).sum();
    let strips = counts.iter().any(|c| c.1);
    let mut ignore = |_| {};
    let spec = RunSpec {
        device,
        cut,
        page,
        penalizer: None,
        limit: Some(SAMPLE),
        cancelled: &|| false,
    };
    let mut run = Run::new(spec, &mut ignore);
    if strips {
        for dir in chapter_dirs {
            if run.pages.len() >= SAMPLE {
                break;
            }
            run.chapter(dir)?;
        }
    } else {
        for (dir, pages) in spread(&chapters, SAMPLE) {
            let files = ChapterFiles {
                pages,
                skipped: Vec::new(),
            };
            run.files(dir, files)?;
        }
    }
    let sampled = run.pages.len().max(1) as u64;
    let average = run.pages.iter().map(|p| p.jpeg.len() as u64).sum::<u64>() / sampled;
    Ok(Estimate {
        pages,
        bytes: pages as u64 * (average + PAGE_OVERHEAD),
    })
}

/// `n` files spread evenly over all chapters' files, grouped by chapter in order.
fn spread<'a>(chapters: &[(&'a Path, Vec<PathBuf>)], n: usize) -> Vec<(&'a Path, Vec<PathBuf>)> {
    let total: usize = chapters.iter().map(|(_, f)| f.len()).sum();
    let picks: Vec<usize> = if total <= n {
        (0..total).collect()
    } else {
        (0..n).map(|k| (2 * k + 1) * total / (2 * n)).collect()
    };
    let mut start = 0;
    let mut out = Vec::new();
    for (dir, files) in chapters {
        let end = start + files.len();
        let mine: Vec<PathBuf> = picks
            .iter()
            .filter(|&&i| (start..end).contains(&i))
            .map(|&i| files[i - start].clone())
            .collect();
        if !mine.is_empty() {
            out.push((*dir, mine));
        }
        start = end;
    }
    out
}

/// Pages one chapter's files give, and whether any is a strip: one page per regular image, and
/// the strips' height (scaled to the screen width) over the cut page height. Unreadable headers
/// count as one page.
fn page_count(files: &[PathBuf], device: &Device, page: &PageSettings) -> (usize, bool) {
    let (mut pages, mut strip_rows) = (0usize, 0f64);
    for file in files {
        match image::image_dimensions(file) {
            Ok((w, h)) if is_long_strip(w, h, device.width, device.height) => {
                strip_rows += h as f64 * device.width.min(w) as f64 / w as f64;
            }
            _ => pages += 1,
        }
    }
    let height = page.page_height(device) as f64;
    (
        pages + (strip_rows / height).ceil() as usize,
        strip_rows > 0.0,
    )
}
