use std::collections::{BTreeSet, HashMap};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::manga_repo::{row_to_manga, COLUMNS};
use super::{Library, ReadingStatus};
use crate::domain::{parse_number, Manga, Result};

/// A followed title with what the user did with it: the library screen's row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryEntry {
    #[serde(flatten)]
    pub manga: Manga,
    pub reading_status: ReadingStatus,
    /// Ids of the categories the title sits on.
    pub categories: Vec<i64>,
    /// When the title was followed (RFC 3339, UTC).
    pub added_at: String,
    /// When a sync last found a chapter the library didn't have (RFC 3339, UTC).
    pub updated_at: String,
    pub chapters: ChapterCounts,
}

/// Chapter numbers of a title, each counted once whatever the number of languages listing it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapterCounts {
    pub total: usize,
    pub downloaded: usize,
    pub exported: usize,
    /// Downloadable numbers with no exported copy: the app has no reader, exported means read.
    pub unread: usize,
}

/// One stored chapter, reduced to what [`count_chapters`] needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterMark {
    pub id: String,
    pub number: String,
    pub available: bool,
    pub downloaded: bool,
    pub exported: bool,
}

/// "12" and "12.0" are the same chapter; labels that aren't numbers compare as written, and a
/// chapter without a number is only ever itself.
fn number_key(mark: &ChapterMark) -> (bool, String) {
    match mark.number.trim() {
        "" => (true, mark.id.clone()),
        n => (
            false,
            parse_number(n).map_or_else(|| n.to_string(), |v| v.to_string()),
        ),
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
    }
}

type Key = (String, String);

/// Every followed title with its status, categories and chapter counts, most recently followed
/// first.
pub fn list_entries(lib: &Library) -> Result<Vec<LibraryEntry>> {
    let conn = lib.conn()?;
    let mut categories = categories_by_manga(&conn)?;
    let marks = marks_by_manga(&conn)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS}, reading_status,
                strftime('%Y-%m-%dT%H:%M:%SZ', added_at),
                strftime('%Y-%m-%dT%H:%M:%SZ', COALESCE(updated_at, added_at))
         FROM manga ORDER BY rowid DESC"
    ))?;
    let rows = stmt.query_map([], |r| {
        Ok((
            row_to_manga(r)?,
            ReadingStatus::parse(&r.get::<_, String>(10)?),
            r.get::<_, Option<String>>(11)?.unwrap_or_default(),
            r.get::<_, Option<String>>(12)?.unwrap_or_default(),
        ))
    })?;
    let mut entries = Vec::new();
    for row in rows {
        let (manga, reading_status, added_at, updated_at) = row?;
        let key = (manga.source.clone(), manga.id.0.clone());
        let chapters = marks
            .get(&key)
            .map(|m| count_chapters(m))
            .unwrap_or_default();
        entries.push(LibraryEntry {
            categories: categories.remove(&key).unwrap_or_default(),
            manga,
            reading_status,
            added_at,
            updated_at,
            chapters,
        });
    }
    Ok(entries)
}

fn categories_by_manga(conn: &Connection) -> Result<HashMap<Key, Vec<i64>>> {
    let mut stmt = conn
        .prepare("SELECT source, manga_id, category_id FROM manga_category ORDER BY category_id")?;
    let rows = stmt.query_map([], |r| Ok(((r.get(0)?, r.get(1)?), r.get(2)?)))?;
    let mut map: HashMap<Key, Vec<i64>> = HashMap::new();
    for row in rows {
        let (key, id) = row?;
        map.entry(key).or_default().push(id);
    }
    Ok(map)
}

fn marks_by_manga(conn: &Connection) -> Result<HashMap<Key, Vec<ChapterMark>>> {
    let mut stmt = conn.prepare(
        "SELECT source, manga_id, number, availability = 'available',
                local_path IS NOT NULL, exported_at IS NOT NULL, id
         FROM chapter",
    )?;
    let rows = stmt.query_map([], |r| {
        let mark = ChapterMark {
            id: r.get(6)?,
            number: r.get(2)?,
            available: r.get(3)?,
            downloaded: r.get(4)?,
            exported: r.get(5)?,
        };
        Ok(((r.get(0)?, r.get(1)?), mark))
    })?;
    let mut map: HashMap<Key, Vec<ChapterMark>> = HashMap::new();
    for row in rows {
        let (key, mark) = row?;
        map.entry(key).or_default().push(mark);
    }
    Ok(map)
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
            }
        );
        assert_eq!(count_chapters(&[]), ChapterCounts::default());
    }

    #[test]
    fn chapters_without_a_number_each_count() {
        let unnumbered = |id: &str| ChapterMark {
            id: id.into(),
            number: " ".into(),
            available: true,
            downloaded: false,
            exported: false,
        };
        let counts = count_chapters(&[unnumbered("a"), unnumbered("b")]);
        assert_eq!((counts.total, counts.unread), (2, 2));
    }
}
