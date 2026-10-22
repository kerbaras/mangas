//! How big an export will be, before it runs. The pages are counted by the export's own run,
//! strips cut by the same smart cut (see [`Survey`]), so the plan has the pages the writer
//! writes. Their size comes from a few pages encoded for real, spread over the book, since scans
//! and strip art vary a lot from one page and chapter to the next: strip pages sampled while
//! their chapter is cut, regular pages picked from the whole book afterwards.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rayon::prelude::*;
use serde::Serialize;

use super::input::ChapterFiles;
use super::run::{Run, RunSpec};
use super::survey::{Survey, SAMPLE};
use super::surveys::chapter_survey;
use super::Device;
use crate::domain::{Error, Result};
use super::parts::page_cost;
use crate::export::{Format, PageInfo, PageSettings};
use crate::smartcut::Options;

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
    format: Format,
) -> Result<Estimate> {
    estimate_largest(&[chapter_dirs.to_vec()], device, cut, page, format)
}

/// The estimate of the book of `books` with the most pages (every book is counted; only that one
/// has pages encoded): the one that decides the size warning.
pub fn estimate_largest(
    books: &[Vec<PathBuf>],
    device: Device,
    cut: Options,
    page: PageSettings,
    format: Format,
) -> Result<Estimate> {
    let counted: Vec<Counted> = (books.iter())
        .map(|b| count(b, device, cut, page))
        .collect::<Result<_>>()?;
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
        .map(|b| sample(count(b, device, cut, page)?, device, cut, page, format))
        .collect()
}

/// The pages each of `chapter_dirs` gives, counted as [`estimate_each`] counts them.
pub fn chapter_pages(
    chapter_dirs: &[PathBuf],
    device: Device,
    cut: Options,
    page: PageSettings,
) -> Result<Vec<usize>> {
    let surveys = count(chapter_dirs, device, cut, page)?;
    Ok(surveys.chapters.iter().map(|(_, s)| s.pages).collect())
}

/// One book's chapters with their surveys, and the pages they give.
struct Counted<'a> {
    chapters: Vec<(&'a Path, Arc<Survey>)>,
    pages: usize,
}

/// Every chapter of a book surveyed, side by side: cutting their strips is the slow part.
fn count(
    chapter_dirs: &[PathBuf],
    device: Device,
    cut: Options,
    page: PageSettings,
) -> Result<Counted<'_>> {
    let chapters: Vec<(&Path, Arc<Survey>)> = (chapter_dirs.par_iter())
        .map(|d| Ok((d.as_path(), chapter_survey(d, device, cut, page)?)))
        .collect::<Result<_>>()?;
    let pages = chapters.iter().map(|(_, s)| s.pages).sum();
    Ok(Counted { chapters, pages })
}

/// `book`'s strip pages at the sizes their surveys encoded them at, and its regular pages at the
/// average of a few of them spread over the book and encoded for real.
fn sample(
    book: Counted,
    device: Device,
    cut: Options,
    page: PageSettings,
    format: Format,
) -> Result<Estimate> {
    let quality = page.image_settings(&device).quality;
    let mut strips = 0;
    for (dir, s) in &book.chapters {
        let jpegs = (s.strip_jpegs_at(quality)).ok_or_else(|| unsampled(dir, quality))?;
        strips += jpegs.iter().map(|&j| page_cost(format, j)).sum::<u64>();
    }
    let regular: Vec<(&Path, Vec<PathBuf>)> = (book.chapters.iter())
        .map(|(d, s)| (*d, s.regular.clone()))
        .collect();
    let regular_pages = regular.iter().map(|(_, f)| f.len()).sum::<usize>() as u64;
    let average = regular_average(&regular, device, cut, page, format)?;
    Ok(Estimate {
        pages: book.pages,
        bytes: strips + regular_pages * average,
    })
}

fn unsampled(dir: &Path, quality: u8) -> Error {
    Error::Other(format!("{} was not sampled at quality {quality}", dir.display()))
}

/// The average JPEG of [`SAMPLE`] of the regular pages of `chapters`, spread over all of them.
fn regular_average(
    chapters: &[(&Path, Vec<PathBuf>)],
    device: Device,
    cut: Options,
    page: PageSettings,
    format: Format,
) -> Result<u64> {
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
    for (dir, pages) in spread(chapters, SAMPLE) {
        let files = ChapterFiles {
            pages,
            skipped: Vec::new(),
        };
        run.files(dir, files)?;
    }
    let sampled = run.pages.len().max(1) as u64;
    let cost = |p: &PageInfo| page_cost(format, p.bytes as u64);
    Ok(run.pages.info().iter().map(cost).sum::<u64>() / sampled)
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
