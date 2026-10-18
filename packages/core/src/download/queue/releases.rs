//! Which release of each chapter to download when a title lists several.

use super::ops::with_stored_paths;
use crate::domain::{collapse_groups, in_group, Chapter, Result};
use crate::library::Library;

/// The releases to download from `chapters` (fresh from the source or from the library): the
/// title's `group` when it has one, else one per chapter, the release already on disk first
/// (read from the library), then `preferred` groups. The CLI and auto-download both pick here.
pub fn pick_releases(
    lib: &Library,
    source: &str,
    chapters: Vec<Chapter>,
    group: Option<&str>,
    preferred: &[String],
) -> Result<Vec<Chapter>> {
    let chapters = with_stored_paths(lib, source, chapters)?;
    Ok(match group {
        Some(group) => in_group(chapters, Some(group)),
        None => collapse_groups(chapters, preferred),
    })
}
