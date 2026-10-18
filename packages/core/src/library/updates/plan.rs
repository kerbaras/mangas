use std::collections::{BTreeSet, HashSet};

use serde::{Deserialize, Serialize};

use super::UpdateReport;
use crate::domain::{parse_number, Chapter, Manga};
use crate::library::{is_local, LibraryEntry, ReadingStatus};

/// Which followed titles a check refreshes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdatePolicy {
    /// Only titles of this source.
    pub source: Option<String>,
    /// Also titles marked completed or dropped, skipped by default.
    pub include_finished: bool,
}

/// A title to refresh.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub manga: Manga,
    pub auto_download: bool,
    pub chapter_group: Option<String>,
}

/// Splits the library into the titles to check and the number left out: local imports have
/// no source, completed/dropped titles are skipped unless the policy includes them.
pub fn select(entries: &[LibraryEntry], policy: &UpdatePolicy) -> (Vec<Target>, usize) {
    let wanted = |e: &LibraryEntry| {
        !is_local(&e.manga.source)
            && policy.source.as_ref().is_none_or(|s| *s == e.manga.source)
            && (policy.include_finished
                || !matches!(
                    e.reading_status,
                    ReadingStatus::Completed | ReadingStatus::Dropped
                ))
    };
    let targets: Vec<Target> = entries
        .iter()
        .filter(|e| wanted(e))
        .map(|e| Target {
            manga: e.manga.clone(),
            auto_download: e.auto_download,
            chapter_group: e.chapter_group.clone(),
        })
        .collect();
    let skipped = entries.len() - targets.len();
    (targets, skipped)
}

/// What the library held for a title before the check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Known {
    pub chapter_ids: HashSet<String>,
    /// Languages synced before: in a language never synced every chapter is the baseline.
    pub languages: HashSet<String>,
}

/// Chapters of the fetched lists that are new: not stored yet, in a language synced before.
pub fn new_chapters(known: &Known, lists: &[(String, Vec<Chapter>)]) -> Vec<Chapter> {
    lists
        .iter()
        .filter(|(lang, _)| known.languages.contains(lang))
        .flat_map(|(_, chapters)| chapters)
        .filter(|c| !known.chapter_ids.contains(&c.id.0))
        .cloned()
        .collect()
}

/// Totals of a report, as the toast, the notification and the CLI print them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    /// New chapter numbers, each counted once whatever the number of languages.
    pub chapters: usize,
    pub titles: usize,
    /// Distinct providers with at least one failed title.
    pub unreachable: usize,
}

impl Summary {
    /// "12 new chapters in 3 titles", "Up to date", "2 providers unreachable" or both halves.
    pub fn line(&self) -> String {
        let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
        let found = match self.chapters {
            0 => None,
            n => Some(format!(
                "{} in {}",
                plural(n, "new chapter"),
                plural(self.titles, "title")
            )),
        };
        let failed = match self.unreachable {
            0 => None,
            1 => Some("1 provider unreachable".to_string()),
            n => Some(format!("{n} providers unreachable")),
        };
        match (found, failed) {
            (Some(f), Some(u)) => format!("{f} · {u}"),
            (Some(f), None) => f,
            (None, Some(u)) => u,
            (None, None) => "Up to date".to_string(),
        }
    }
}

/// What makes two chapters "the same chapter" across languages: "2" and "2.0" match.
pub(super) fn number_key(c: &Chapter) -> String {
    match parse_number(c.number.trim()) {
        Some(n) => n.to_string(),
        None if c.number.trim().is_empty() => format!("#{}", c.id.0),
        None => c.number.trim().to_string(),
    }
}

pub fn summary(report: &UpdateReport) -> Summary {
    let numbers = |chapters: &[Chapter]| -> usize {
        chapters
            .iter()
            .map(number_key)
            .collect::<BTreeSet<_>>()
            .len()
    };
    let providers: BTreeSet<&str> = report.failures.iter().map(|f| f.source.as_str()).collect();
    Summary {
        chapters: report.updates.iter().map(|u| numbers(&u.chapters)).sum(),
        titles: report.updates.len(),
        unreachable: providers.len(),
    }
}

/// Title and body of the desktop notification for a run, `None` when nothing is new: one
/// notification per run, naming a few titles.
pub fn notification(report: &UpdateReport) -> Option<(String, String)> {
    const NAMED: usize = 3;
    let totals = summary(report);
    if totals.chapters == 0 {
        return None;
    }
    let titles: Vec<&str> = report.updates.iter().map(|u| u.title.as_str()).collect();
    let body = match titles.len().saturating_sub(NAMED) {
        0 => titles.join(", "),
        more => format!("{} and {more} more", titles[..NAMED].join(", ")),
    };
    let found = Summary {
        unreachable: 0,
        ..totals
    };
    Some((found.line(), body))
}
