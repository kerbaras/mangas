//! What makes an over-budget book fit: the most chapters per book and the highest lower JPEG
//! quality whose estimates stay within the delivery's target, and the part size. Plans offer
//! them, and so does the warning about a written file over its budget.

use std::path::PathBuf;

use super::budget::{BudgetSpec, Levers, QUALITY_STEPS};
use super::estimate::{chapter_pages, estimate_each, Estimate};
use crate::domain::Result;
use crate::export::PageSettings;

/// The levers for the book of `dirs` under `spec`, as its plan offers them when it is over: for
/// a written file over its budget, which the plan may not have seen coming.
pub fn book_levers(dirs: &[PathBuf], spec: BudgetSpec) -> Result<Levers> {
    let book = [dirs.to_vec()];
    let e = estimate_each(&book, spec.device, spec.cut, spec.page, spec.format)?;
    let e = (e.into_iter().next()).unwrap_or(Estimate { pages: 0, bytes: 0 });
    levers(
        dirs,
        &e,
        spec.page.image_settings(&spec.device).quality,
        spec,
    )
}

pub(super) fn levers(
    dirs: &[PathBuf],
    e: &Estimate,
    quality: u8,
    spec: BudgetSpec,
) -> Result<Levers> {
    let target = spec.delivery.target();
    Ok(Levers {
        split_bytes: target,
        chapters_per_book: chapters_per_book(dirs, e, spec)?,
        jpeg_quality: fitting_quality(dirs, quality, spec)?,
    })
}

/// The most chapters per book of `dirs` that keeps every book within the target: guessed from
/// each chapter's pages at the book's average page size, then checked on every book, since a
/// shorter book of bigger pages can be the larger one.
fn chapters_per_book(dirs: &[PathBuf], e: &Estimate, spec: BudgetSpec) -> Result<Option<usize>> {
    let target = spec.delivery.target();
    let pages = chapter_pages(dirs, &spec.device, &spec.page);
    let mut guess = most_that_fit(&pages, e.bytes / e.pages.max(1) as u64, target);
    while let Some(n) = guess {
        let books: Vec<Vec<PathBuf>> = dirs.chunks(n).map(<[PathBuf]>::to_vec).collect();
        let each = estimate_each(&books, spec.device, spec.cut, spec.page, spec.format)?;
        if each.iter().all(|e| e.bytes <= target) {
            return Ok(Some(n));
        }
        guess = (n > 1).then(|| n - 1);
    }
    Ok(None)
}

/// The most chapters of `pages` per book, fewer than all, whose books all stay within `target`.
pub(super) fn most_that_fit(pages: &[usize], page_bytes: u64, target: u64) -> Option<usize> {
    let fits =
        |n: usize| (pages.chunks(n)).all(|c| c.iter().sum::<usize>() as u64 * page_bytes <= target);
    (1..pages.len()).rev().find(|&n| fits(n))
}

/// The highest of [`QUALITY_STEPS`] below `quality` whose estimate is within the target.
fn fitting_quality(dirs: &[PathBuf], quality: u8, spec: BudgetSpec) -> Result<Option<u8>> {
    let book = [dirs.to_vec()];
    for q in QUALITY_STEPS.into_iter().filter(|&q| q < quality) {
        let page = PageSettings {
            quality: Some(q),
            ..spec.page
        };
        let e = estimate_each(&book, spec.device, spec.cut, page, spec.format)?;
        if e.first().is_some_and(|e| e.bytes <= spec.delivery.target()) {
            return Ok(Some(q));
        }
    }
    Ok(None)
}
