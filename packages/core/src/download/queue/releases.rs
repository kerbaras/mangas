//! Which release of each chapter to download when a title lists several.

use super::ops::{still_on_disk, with_stored_paths};
use crate::domain::{collapse_groups, in_group, Chapter, Result};
use crate::library::Library;

/// The releases to download from `chapters` (fresh from the source or from the library): the
/// title's `group` when it has one, else one per chapter, the release with a folder on disk
/// first (read from the library), then `preferred` groups. The CLI and auto-download both pick here.
pub fn pick_releases(
    lib: &Library,
    source: &str,
    chapters: Vec<Chapter>,
    group: Option<&str>,
    preferred: &[String],
) -> Result<Vec<Chapter>> {
    // A partial folder of a downloadable release resumes; one that cannot be fetched again
    // only counts when it is complete.
    let chapters = with_stored_paths(lib, source, chapters)?
        .into_iter()
        .flat_map(|c| match c.is_available() {
            true => vec![c],
            false => still_on_disk(vec![c]),
        })
        .collect();
    Ok(match group {
        Some(group) => in_group(chapters, Some(group)),
        None => collapse_groups(chapters, preferred),
    })
}
