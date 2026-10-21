use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::domain::parse_number;

/// Chapter numbers of a title, each counted once whatever the number of languages listing it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapterCounts {
    pub total: usize,
    pub downloaded: usize,
    pub exported: usize,
    /// Downloadable numbers with no exported copy: the app has no reader, exported means read.
    pub unread: usize,
    /// Numbers first stored after the user last opened the title: the "new" badge.
    pub new: usize,
}

/// One stored chapter, reduced to what [`count_chapters`] needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterMark {
    pub id: String,
    pub number: String,
    pub available: bool,
    pub downloaded: bool,
    pub exported: bool,
    /// First stored after the user last opened the title.
    pub new: bool,
}

/// "12" and "12.0" are the same chapter; labels that aren't numbers compare as written, and a
/// chapter without a number is only ever itself.
fn number_key(mark: &ChapterMark) -> (bool, String) {
    key_of(&mark.number, &mark.id)
}

fn key_of(number: &str, id: &str) -> (bool, String) {
    match number.trim() {
        "" => (true, id.to_string()),
        n => (
            false,
            parse_number(n).map_or_else(|| n.to_string(), |v| v.to_string()),
        ),
    }
}

/// [`number_key`] as one string, for the counts query: chapters with equal keys are one number.
pub(crate) fn chapter_key(number: &str, id: &str) -> String {
    match key_of(number, id) {
        (true, id) => format!("id:{id}"),
        (false, n) => format!("no:{n}"),
    }
}

pub fn count_chapters(marks: &[ChapterMark]) -> ChapterCounts {
    let numbers = |keep: fn(&ChapterMark) -> bool| -> BTreeSet<(bool, String)> {
        marks.iter().filter(|m| keep(m)).map(number_key).collect()
    };
    let exported = numbers(|m| m.exported);
    ChapterCounts {
        total: numbers(|_| true).len(),
        downloaded: numbers(|m| m.downloaded).len(),
        unread: numbers(|m| m.available).difference(&exported).count(),
        exported: exported.len(),
        new: numbers(|m| m.new).len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(number: &str, available: bool, downloaded: bool, exported: bool) -> ChapterMark {
        ChapterMark {
            id: format!("c{number}"),
            number: number.into(),
            available,
            downloaded,
            exported,
            new: false,
        }
    }

    #[test]
    fn numbers_count_once_across_languages_and_exported_is_read() {
        let marks = [
            mark("1", true, true, true),
            mark("1.0", true, false, false), // same chapter in another language
            mark("2", true, true, false),
            mark("3", true, false, false),
            mark("4", false, false, false), // locked: listed, never unread
            mark("Oneshot", true, false, false),
        ];
        let counts = count_chapters(&marks);
        assert_eq!(
            counts,
            ChapterCounts {
                total: 5,
                downloaded: 2,
                exported: 1,
                unread: 3,
                new: 0,
            }
        );
        assert_eq!(count_chapters(&[]), ChapterCounts::default());
    }

    #[test]
    fn new_badge_counts_numbers_once() {
        let new = |id: &str, number: &str| ChapterMark {
            new: true,
            id: id.into(),
            ..mark(number, true, false, false)
        };
        let marks = [
            mark("1", true, false, false),
            new("a", "2"),
            new("b", "2.0"),
            new("c", "3"),
        ];
        assert_eq!(count_chapters(&marks).new, 2);
    }

    #[test]
    fn chapters_without_a_number_each_count() {
        let unnumbered = |id: &str| ChapterMark {
            id: id.into(),
            number: " ".into(),
            available: true,
            downloaded: false,
            exported: false,
            new: true,
        };
        let counts = count_chapters(&[unnumbered("a"), unnumbered("b")]);
        assert_eq!((counts.total, counts.unread, counts.new), (2, 2, 2));
    }
}
