use std::collections::HashMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::counts::{count_chapters, ChapterCounts, ChapterMark};
use super::manga_repo::{row_to_manga, COLUMNS};
use super::{Library, ReadingStatus};
use crate::domain::{Manga, Result};

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
    /// New chapters of this title are queued for download by update checks.
    pub auto_download: bool,
    /// The scanlation group the title is narrowed to; `None` for every group.
    #[serde(default)]
    pub chapter_group: Option<String>,
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
                strftime('%Y-%m-%dT%H:%M:%SZ', COALESCE(updated_at, added_at)), auto_download,
                chapter_group
         FROM manga ORDER BY rowid DESC"
    ))?;
    let rows = stmt.query_map([], |r| {
        Ok((
            row_to_manga(r)?,
            ReadingStatus::parse(&r.get::<_, String>(11)?),
            r.get::<_, Option<String>>(12)?.unwrap_or_default(),
            r.get::<_, Option<String>>(13)?.unwrap_or_default(),
            r.get::<_, bool>(14)?,
            r.get::<_, Option<String>>(15)?,
        ))
    })?;
    let mut entries = Vec::new();
    for row in rows {
        let (manga, reading_status, added_at, updated_at, auto_download, chapter_group) = row?;
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
            auto_download,
            chapter_group,
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

/// Chapter marks per title, of its saved group only when it is narrowed to one.
fn marks_by_manga(conn: &Connection) -> Result<HashMap<Key, Vec<ChapterMark>>> {
    let mut stmt = conn.prepare(
        "SELECT c.source, c.manga_id, c.number, c.availability = 'available',
                c.local_path IS NOT NULL, c.exported_at IS NOT NULL, c.id,
                c.first_seen_at IS NOT NULL
                  AND c.first_seen_at > COALESCE(m.last_viewed_at, '')
         FROM chapter c JOIN manga m ON m.source = c.source AND m.id = c.manga_id
         WHERE m.chapter_group IS NULL OR c.scanlation_group = m.chapter_group",
    )?;
    let rows = stmt.query_map([], |r| {
        let mark = ChapterMark {
            id: r.get(6)?,
            number: r.get(2)?,
            available: r.get(3)?,
            downloaded: r.get(4)?,
            exported: r.get(5)?,
            new: r.get(7)?,
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
