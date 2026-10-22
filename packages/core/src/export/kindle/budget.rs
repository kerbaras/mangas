//! The size budget a book is planned against for its delivery: Send to Kindle takes 50 MB per
//! file by email and 200 MB on the web, so a book is sized before anything is written, with the
//! two levers that make it fit (fewer chapters per book, a lower JPEG quality). Nothing is
//! re-encoded unless one of them is picked.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::estimate::{estimate_each, Estimate};
use super::parts::SEND_TO_KINDLE_LIMIT;
use super::Device;
use crate::domain::Result;
use crate::export::PageSettings;
use crate::smartcut::Options;

/// Amazon's Send-to-Kindle web uploader limit for one file, in bytes.
pub const SEND_TO_KINDLE_WEB_LIMIT: u64 = 200_000_000;
/// The lower JPEG qualities a plan offers, highest first (the desktop picker's choices).
pub const QUALITY_STEPS: [u8; 4] = [80, 70, 60, 50];

/// How the books leave the computer, which decides their budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    /// Emailed to the Kindle: 50 MB per file.
    Email,
    /// Saved, copied over USB or uploaded on the web: 200 MB per file.
    #[default]
    Other,
}

impl Delivery {
    /// Bytes one file may have.
    pub const fn budget(self) -> u64 {
        match self {
            Delivery::Email => SEND_TO_KINDLE_LIMIT,
            Delivery::Other => SEND_TO_KINDLE_WEB_LIMIT,
        }
    }

    /// What a lever aims for: nine tenths of the budget, room for what the estimate misses
    /// (45 MB by email, the part size #134 offers).
    pub const fn target(self) -> u64 {
        self.budget() / 10 * 9
    }
}

/// One planned book sized against its budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookBudget {
    pub pages: usize,
    pub estimated_bytes: u64,
    pub budget_bytes: u64,
    /// The biggest file the book is written as (a part, when split) is estimated over budget.
    pub over_budget: bool,
    /// JPEG quality of the pages, as picked or the profile's.
    pub jpeg_quality: u8,
    /// Files the book is estimated to be written as: its parts when split, else 1.
    pub files: usize,
    /// What makes an over-budget book fit; absent when it fits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub levers: Option<Levers>,
}

/// The ways to bring an over-budget book under its budget, each an explicit choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Levers {
    /// Write the book as parts of at most this many bytes (the #134 split).
    pub split_bytes: u64,
    /// Books of at most this many chapters are estimated to fit; absent for one chapter.
    pub chapters_per_book: Option<usize>,
    /// The highest lower JPEG quality estimated to fit; absent when none of the steps does.
    pub jpeg_quality: Option<u8>,
}

/// What a plan is sized for: the device and page choices, the delivery and the part size.
#[derive(Debug, Clone, Copy)]
pub struct BudgetSpec {
    pub device: Device,
    pub cut: Options,
    pub page: PageSettings,
    pub delivery: Delivery,
    /// The `--split-mb` part size in bytes, when the books are written in parts.
    pub split: Option<u64>,
}

/// Each of `books` (its chapter folders) estimated and held against `spec`'s budget.
pub fn plan_budget(books: &[Vec<PathBuf>], spec: BudgetSpec) -> Result<Vec<BookBudget>> {
    let estimates = estimate_each(books, spec.device, spec.cut, spec.page)?;
    let quality = spec.page.image_settings(&spec.device).quality;
    books
        .iter()
        .zip(estimates)
        .map(|(dirs, e)| {
            let over = biggest_file(&e, spec.split) > spec.delivery.budget();
            let levers = match over {
                true => Some(levers(dirs, &e, quality, spec)?),
                false => None,
            };
            Ok(BookBudget {
                pages: e.pages,
                estimated_bytes: e.bytes,
                budget_bytes: spec.delivery.budget(),
                over_budget: over,
                jpeg_quality: quality,
                files: files(&e, spec.split),
                levers,
            })
        })
        .collect()
}

/// The biggest file a book of `e` is written as: a part of `split` at most, when split.
fn biggest_file(e: &Estimate, split: Option<u64>) -> u64 {
    split.map_or(e.bytes, |s| e.bytes.min(s))
}

/// Files a book of `e` is written as: one per `split` bytes, never more than its pages.
fn files(e: &Estimate, split: Option<u64>) -> usize {
    let parts = split.map_or(1, |s| e.bytes.div_ceil(s.max(1)) as usize);
    parts.clamp(1, e.pages.max(1))
}

fn levers(dirs: &[PathBuf], e: &Estimate, quality: u8, spec: BudgetSpec) -> Result<Levers> {
    let target = spec.delivery.target();
    Ok(Levers {
        split_bytes: target,
        chapters_per_book: chapters_per_book(dirs.len(), e.bytes, target),
        jpeg_quality: fitting_quality(dirs, quality, spec)?,
    })
}

/// How many of `chapters` a book of `bytes` holds within `target`, as chapters average out.
fn chapters_per_book(chapters: usize, bytes: u64, target: u64) -> Option<usize> {
    let per = chapters as u128 * target as u128 / bytes.max(1) as u128;
    (chapters > 1).then(|| (per as usize).clamp(1, chapters - 1))
}

/// The highest of [`QUALITY_STEPS`] below `quality` whose estimate is within the target.
fn fitting_quality(dirs: &[PathBuf], quality: u8, spec: BudgetSpec) -> Result<Option<u8>> {
    let book = [dirs.to_vec()];
    for q in QUALITY_STEPS.into_iter().filter(|&q| q < quality) {
        let page = PageSettings {
            quality: Some(q),
            ..spec.page
        };
        let e = estimate_each(&book, spec.device, spec.cut, page)?;
        if e.first().is_some_and(|e| e.bytes <= spec.delivery.target()) {
            return Ok(Some(q));
        }
    }
    Ok(None)
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
