//! How big an export will be, before it runs: a few pages are encoded for real and their
//! average size is scaled to the page count read from the image headers. Books of regular pages
//! sample pages spread over every chapter, since scans vary a lot between chapters; webtoon
//! strips sample the first pages, as a strip only cuts in order.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::input::{chapter_files, ChapterFiles};
use super::parts::page_cost;
use super::run::{Run, RunSpec};
use super::Device;
use crate::domain::Result;
use crate::export::{Format, PageInfo, PageSettings};
use crate::smartcut::{is_long_strip, Options};

/// Pages encoded to measure the average page.
const SAMPLE: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Estimate {
    pub pages: usize,
    pub bytes: u64,
}

/// The estimated size of the `format` book `chapter_dirs` make on `device` with `cut` and `page`.
pub fn estimate(
    chapter_dirs: &[PathBuf],
    device: Device,
    cut: Options,
    page: PageSettings,
    format: Format,
) -> Result<Estimate> {
    estimate_largest(&[chapter_dirs.to_vec()], device, cut, page, format)
}

/// The estimate of the book of `books` with the most pages (counted from the image headers of
/// every book; only that one has pages encoded): the one that decides the size warning.
pub fn estimate_largest(
    books: &[Vec<PathBuf>],
    device: Device,
    cut: Options,
    page: PageSettings,
    format: Format,
) -> Result<Estimate> {
    let counted: Vec<Counted> = books.iter().map(|b| count(b, &device, &page)).collect();
    match counted.into_iter().max_by_key(|c| c.pages) {
        Some(book) => sample(book, device, cut, page, format),
        None => Ok(Estimate { pages: 0, bytes: 0 }),
    }
}

/// The estimate of each of `books`: every book is counted and has its own pages sampled.
pub fn estimate_each(
    books: &[Vec<PathBuf>],
    device: Device,
    cut: Options,
    page: PageSettings,
    format: Format,
) -> Result<Vec<Estimate>> {
    (books.iter())
        .map(|b| sample(count(b, &device, &page), device, cut, page, format))
        .collect()
}

/// The pages each of `chapter_dirs` gives, counted as [`estimate_each`] counts them.
pub fn chapter_pages(chapter_dirs: &[PathBuf], device: &Device, page: &PageSettings) -> Vec<usize> {
    let counted = count(chapter_dirs, device, page);
    (counted.chapters.iter())
        .map(|(_, files)| page_count(files, device, page).0)
        .collect()
}

/// One book's chapters with their files, and the pages they give.
struct Counted<'a> {
    dirs: &'a [PathBuf],
    chapters: Vec<(&'a Path, Vec<PathBuf>)>,
    pages: usize,
    strips: bool,
}

fn count<'a>(chapter_dirs: &'a [PathBuf], device: &Device, page: &PageSettings) -> Counted<'a> {
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
        .map(|(_, files)| page_count(files, device, page))
        .collect();
    Counted {
        dirs: chapter_dirs,
        pages: counts.iter().map(|c| c.0).sum(),
        strips: counts.iter().any(|c| c.1),
        chapters,
    }
}

/// `book`'s pages times the average cost of a few of them encoded for real.
fn sample(
    book: Counted,
    device: Device,
    cut: Options,
    page: PageSettings,
    format: Format,
) -> Result<Estimate> {
    let Counted {
        dirs: chapter_dirs,
        chapters,
        pages,
        strips,
    } = book;
    let mut ignore = |_| {};
    let spec = RunSpec {
        device,
        cut,
        page,
        penalizer: None,
        limit: Some(SAMPLE),
        cancelled: &|| false,
        spool_dir: None,
    };
    let mut run = Run::new(spec, &mut ignore)?;
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
    let cost = |p: &PageInfo| page_cost(format, p.bytes as u64);
    let average = run.pages.info().iter().map(cost).sum::<u64>() / sampled;
    Ok(Estimate {
        pages,
        bytes: pages as u64 * average,
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
