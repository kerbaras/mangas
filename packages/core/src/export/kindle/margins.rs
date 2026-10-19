//! Side margins of a chapter's strips, so cropped chunks still stitch together.

use std::collections::HashMap;
use std::path::PathBuf;

use rayon::prelude::*;

use super::input::decode;
use super::trim::{narrower, side_margins};
use crate::domain::Result;

/// Side margins blank in every image of a width, so strip chunks keep stitching together.
/// Margin scans look at this many files at most, spread over the chapter, so a preview of a long
/// chapter doesn't decode every image first (and export and preview still agree).
const MARGIN_SAMPLE: usize = 24;

fn margin_sample(files: &[PathBuf]) -> Vec<&PathBuf> {
    let step = files.len().div_ceil(MARGIN_SAMPLE).max(1);
    files.iter().step_by(step).collect()
}

pub(super) fn strip_margins(files: &[PathBuf]) -> Result<HashMap<u32, (u32, u32)>> {
    let found: Vec<(u32, Option<(u32, u32)>)> = margin_sample(files)
        .par_iter()
        .map(|f| decode(f).map(|img| (img.width(), side_margins(&img))))
        .collect::<Result<_>>()?;
    let mut by_width: HashMap<u32, Option<(u32, u32)>> = HashMap::new();
    for (w, m) in found {
        let entry = by_width.entry(w).or_insert(m);
        *entry = narrower(*entry, m);
    }
    Ok(by_width
        .into_iter()
        .filter_map(|(w, m)| m.map(|m| (w, m)))
        .collect())
}
