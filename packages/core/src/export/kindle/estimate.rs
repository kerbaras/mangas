//! How big an export will be, before it runs: the first pages are encoded for real and their
//! average size is scaled to the page count read from the image headers.

use std::path::PathBuf;

use serde::Serialize;

use super::input::chapter_files;
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
    let pages: usize = chapter_dirs.iter().map(|d| page_count(d, &device, &page)).sum();
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
    for dir in chapter_dirs {
        if run.pages.len() >= SAMPLE {
            break;
        }
        run.chapter(dir)?;
    }
    let sampled = run.pages.len().max(1) as u64;
    let average = run.pages.iter().map(|p| p.jpeg.len() as u64).sum::<u64>() / sampled;
    Ok(Estimate {
        pages,
        bytes: pages as u64 * (average + PAGE_OVERHEAD),
    })
}

/// Pages one chapter folder gives: one per regular image, and its strips' height (scaled to
/// the screen width) over the cut page height. Unreadable headers count as one page.
fn page_count(dir: &std::path::Path, device: &Device, page: &PageSettings) -> usize {
    let Ok(files) = chapter_files(dir) else {
        return 0;
    };
    let (mut pages, mut strip_rows) = (0usize, 0f64);
    for file in &files.pages {
        match image::image_dimensions(file) {
            Ok((w, h)) if is_long_strip(w, h, device.width, device.height) => {
                strip_rows += h as f64 * device.width.min(w) as f64 / w as f64;
            }
            _ => pages += 1,
        }
    }
    let height = page.page_height(device) as f64;
    pages + (strip_rows / height).ceil() as usize
}
