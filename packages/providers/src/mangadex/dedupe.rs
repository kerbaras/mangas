//! MangaDex lists one upload per group for the same chapter. The source returns them all; core
//! narrows them to the title's group, or to one per number when none is picked.

use mangas_core::domain::{collapse_groups, sort_by_number, Chapter};

/// A mapped feed chapter plus when it was published.
#[derive(Debug, Clone)]
pub struct FeedChapter {
    pub chapter: Chapter,
    /// `publishAt` as sent (UTC RFC 3339, comparable as text).
    pub published_at: String,
}

/// Every upload sorted by chapter number, the most recent first within a number.
pub fn releases(mut entries: Vec<FeedChapter>) -> Vec<Chapter> {
    entries.sort_by(|a, b| b.published_at.cmp(&a.published_at));
    sort_by_number(entries.into_iter().map(|e| e.chapter).collect())
}

/// One upload per chapter number, as core picks it when the title has no group: downloadable
/// first, then `preferred_groups` order, then the most recent.
pub fn dedupe_chapters(entries: Vec<FeedChapter>, preferred_groups: &[String]) -> Vec<Chapter> {
    collapse_groups(releases(entries), preferred_groups)
}
