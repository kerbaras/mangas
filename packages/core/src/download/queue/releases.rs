//! Which release of each chapter to download when a title lists several.

use super::ops::{still_on_disk, with_stored_paths};
use crate::domain::{collapse_groups, in_group, same_slot, Chapter, Result};
use crate::library::Library;

/// The releases to download from `chapters` (fresh from the source or from the library): the
/// title's `group` when it has one, else one per chapter, the release with a folder on disk
/// first (read from the library; a complete one over a partial one), then `preferred` groups. The CLI and auto-download both pick here.
pub fn pick_releases(
    lib: &Library,
    source: &str,
    chapters: Vec<Chapter>,
    group: Option<&str>,
    preferred: &[String],
) -> Result<Vec<Chapter>> {
    let stored = with_stored_paths(lib, source, chapters)?;
    let complete = still_on_disk(stored.clone());
    let has_complete = |c: &Chapter| {
        complete
            .iter()
            .any(|o| o.local_path.is_some() && same_slot(o, c))
    };
    // A complete folder counts; a partial one only for a downloadable release (it resumes)
    // with no complete release of the same chapter.
    let chapters = stored
        .into_iter()
        .zip(&complete)
        .map(|(c, done)| match done.local_path.is_some() {
            true => c,
            false if c.local_path.is_some() && c.is_available() && !has_complete(&c) => c,
            false => Chapter {
                local_path: None,
                ..c
            },
        })
        .collect();
    Ok(match group {
        Some(group) => in_group(chapters, Some(group)),
        None => collapse_groups(chapters, preferred),
    })
}
