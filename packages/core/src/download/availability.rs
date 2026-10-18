//! Which selected chapters can be downloaded. Locked and externally hosted chapters are listed
//! by their source but have no pages to fetch.

use crate::domain::{Availability, Chapter, Error, Result};

/// Splits `chapters` into the downloadable ones and the skipped ones, both in their order.
/// Chapters already on disk always count as downloadable: a re-download only resumes them.
pub fn split_available(chapters: Vec<Chapter>) -> (Vec<Chapter>, Vec<Chapter>) {
    chapters
        .into_iter()
        .partition(|c| c.is_available() || c.is_downloaded())
}

/// [`Error::Unavailable`] naming the reason when `chapter` cannot be downloaded.
pub fn ensure_available(chapter: &Chapter) -> Result<()> {
    match chapter.availability.reason() {
        Some(reason) if !chapter.is_downloaded() => Err(Error::Unavailable {
            number: label(chapter),
            reason,
        }),
        _ => Ok(()),
    }
}

/// One line for the skipped chapters ("skipped 3 chapters: 2 locked, 1 on another site"), or
/// `None` when nothing was skipped.
pub fn skipped_summary(skipped: &[Chapter]) -> Option<String> {
    if skipped.is_empty() {
        return None;
    }
    let locked = skipped
        .iter()
        .filter(|c| c.availability == Availability::Locked)
        .count();
    let external = skipped.len() - locked;
    let parts: Vec<String> = [(locked, "locked"), (external, "on another site")]
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, what)| format!("{n} {what}"))
        .collect();
    let noun = if skipped.len() == 1 {
        "chapter"
    } else {
        "chapters"
    };
    Some(format!(
        "skipped {} {noun}: {}",
        skipped.len(),
        parts.join(", ")
    ))
}

fn label(chapter: &Chapter) -> String {
    if chapter.number.is_empty() {
        chapter.id.0.clone()
    } else {
        chapter.number.clone()
    }
}
