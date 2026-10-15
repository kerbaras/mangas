//! Chapter selection by published number: `"1-5,7"` means chapters 1 to 5 and chapter 7.

use crate::domain::{Chapter, Error, Result};

/// Inclusive numeric ranges parsed from a selection spec.
pub fn parse_selection(spec: &str) -> Result<Vec<(f64, f64)>> {
    spec.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| parse_range(part).ok_or_else(|| invalid(spec)))
        .collect()
}

/// Chapters whose number falls inside the selection, in their original order.
pub fn select_chapters(chapters: &[Chapter], spec: &str) -> Result<Vec<Chapter>> {
    let ranges = parse_selection(spec)?;
    if ranges.is_empty() {
        return Err(invalid(spec));
    }
    Ok(chapters
        .iter()
        .filter(|c| {
            c.number_value()
                .map(|n| ranges.iter().any(|(lo, hi)| *lo <= n && n <= *hi))
                .unwrap_or(false)
        })
        .cloned()
        .collect())
}

fn parse_range(part: &str) -> Option<(f64, f64)> {
    match part.split_once('-') {
        Some((lo, hi)) => {
            let (lo, hi) = (parse_bound(lo)?, parse_bound(hi)?);
            (lo <= hi).then_some((lo, hi))
        }
        None => {
            let n = parse_bound(part)?;
            Some((n, n))
        }
    }
}

fn parse_bound(s: &str) -> Option<f64> {
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && *n >= 0.0)
}

fn invalid(spec: &str) -> Error {
    Error::Other(format!(
        "invalid chapter selection '{spec}' (expected numbers or ranges like 1-5,7)"
    ))
}
