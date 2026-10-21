use std::collections::HashMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::count_query::{counts_by_manga, Key};
use super::counts::ChapterCounts;
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

/// Every followed title with its status, categories and chapter counts, most recently followed
/// first.
pub fn list_entries(lib: &Library) -> Result<Vec<LibraryEntry>> {
    let conn = lib.conn()?;
    let mut categories = categories_by_manga(&conn)?;
    let mut counts = counts_by_manga(&conn)?;
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
            ReadingStatus::parse(&r.get::<_, String>(10)?),
            r.get::<_, Option<String>>(11)?.unwrap_or_default(),
            r.get::<_, Option<String>>(12)?.unwrap_or_default(),
            r.get::<_, bool>(13)?,
            r.get::<_, Option<String>>(14)?,
        ))
    })?;
    let mut entries = Vec::new();
    for row in rows {
        let (manga, reading_status, added_at, updated_at, auto_download, chapter_group) = row?;
        let key = (manga.source.clone(), manga.id.0.clone());
        let chapters = counts.remove(&key).unwrap_or_default();
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
