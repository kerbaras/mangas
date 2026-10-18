//! One chapter per number when several groups uploaded the same chapter.

use std::collections::HashMap;

use mangas_core::domain::{sort_by_number, Availability, Chapter};

/// A mapped feed chapter plus what dedupe ranks it by.
#[derive(Debug, Clone)]
pub struct FeedChapter {
    pub chapter: Chapter,
    /// `publishAt` as sent (UTC RFC 3339, comparable as text).
    pub published_at: String,
}

/// Keeps one upload per chapter number and sorts by number. A downloadable upload beats an
/// externally hosted one, then an upload by a preferred group (case-insensitive; earlier in
/// `preferred_groups` wins), then the most recent `publishAt`. Chapters without a number (oneshots, extras) are all kept.
pub fn dedupe_chapters(entries: Vec<FeedChapter>, preferred_groups: &[String]) -> Vec<Chapter> {
    let mut best: HashMap<String, FeedChapter> = HashMap::new();
    let mut unnumbered = Vec::new();
    for entry in entries {
        if entry.chapter.number.is_empty() {
            unnumbered.push(entry.chapter);
            continue;
        }
        let key = entry.chapter.number.clone();
        let wins = best
            .get(&key)
            .is_none_or(|kept| rank(&entry, preferred_groups) < rank(kept, preferred_groups));
        if wins {
            best.insert(key, entry);
        }
    }
    let chapters = best.into_values().map(|e| e.chapter).chain(unnumbered);
    sort_by_number(chapters.collect())
}

/// Lower is better: downloadable first, then a readable link, preferred-group position, then
/// newest first.
fn rank<'a>(
    entry: &'a FeedChapter,
    preferred: &[String],
) -> (u8, usize, std::cmp::Reverse<&'a str>) {
    let group = preference(entry.chapter.group.as_deref(), preferred);
    let tier = match &entry.chapter.availability {
        Availability::Available => 0,
        Availability::External { url: Some(_) } => 1,
        _ => 2,
    };
    (tier, group, std::cmp::Reverse(entry.published_at.as_str()))
}

fn preference(groups: Option<&str>, preferred: &[String]) -> usize {
    let names: Vec<&str> = groups.map_or_else(Vec::new, |g| g.split(", ").collect());
    preferred
        .iter()
        .position(|p| {
            names
                .iter()
                .any(|n| n.trim().eq_ignore_ascii_case(p.trim()))
        })
        .unwrap_or(usize::MAX)
}
