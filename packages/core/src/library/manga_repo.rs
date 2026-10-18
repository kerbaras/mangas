use rusqlite::{params, Connection, OptionalExtension, Row};

use super::Library;
use crate::domain::{Manga, MangaId, MangaStatus, Result};

pub(super) const COLUMNS: &str =
    "id, source, title, description, cover_url, status, authors, artists, year, tags";

/// Inserts the manga or refreshes its metadata if it is already followed.
pub fn upsert_manga(lib: &Library, manga: &Manga) -> Result<()> {
    upsert_on(&*lib.conn()?, manga)
}

pub(super) fn upsert_on(conn: &Connection, manga: &Manga) -> Result<()> {
    conn.execute(
        "INSERT INTO manga
            (id, source, title, description, cover_url, status, authors, artists, year, tags)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT (source, id) DO UPDATE SET
            title = excluded.title, description = excluded.description,
            cover_url = excluded.cover_url, status = excluded.status,
            authors = excluded.authors, artists = excluded.artists,
            year = excluded.year, tags = excluded.tags",
        params![
            manga.id.0,
            manga.source,
            manga.title,
            manga.description,
            manga.cover_url,
            status_to_str(manga.status),
            list_to_json(&manga.authors),
            list_to_json(&manga.artists),
            manga.year,
            list_to_json(&manga.tags),
        ],
    )?;
    Ok(())
}

/// All followed manga, most recently followed first (insertion order via rowid).
pub fn list_manga(lib: &Library) -> Result<Vec<Manga>> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM manga ORDER BY rowid DESC"))?;
    let rows = stmt.query_map([], row_to_manga)?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// All followed manga with when they were followed (`YYYY-MM-DD HH:MM:SS`, UTC), oldest first.
pub(crate) fn list_manga_with_added(lib: &Library) -> Result<Vec<(Manga, String)>> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS}, added_at FROM manga ORDER BY rowid"
    ))?;
    let rows = stmt.query_map([], |r| Ok((row_to_manga(r)?, r.get(10)?)))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// One followed manga, if present.
pub fn get_manga(lib: &Library, source: &str, id: &MangaId) -> Result<Option<Manga>> {
    let conn = lib.conn()?;
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM manga WHERE source = ?1 AND id = ?2"),
            params![source, id.0],
            row_to_manga,
        )
        .optional()?)
}

/// Unfollows a manga; its chapter rows go with it. Returns whether a row was removed.
pub fn remove_manga(lib: &Library, source: &str, id: &MangaId) -> Result<bool> {
    let removed = lib.conn()?.execute(
        "DELETE FROM manga WHERE source = ?1 AND id = ?2",
        params![source, id.0],
    )?;
    Ok(removed > 0)
}

pub(super) fn row_to_manga(row: &Row<'_>) -> rusqlite::Result<Manga> {
    Ok(Manga {
        id: MangaId(row.get(0)?),
        source: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        cover_url: row.get(4)?,
        status: status_from_str(&row.get::<_, String>(5)?),
        authors: list_from_json(&row.get::<_, String>(6)?),
        artists: list_from_json(&row.get::<_, String>(7)?),
        year: row.get(8)?,
        tags: list_from_json(&row.get::<_, String>(9)?),
    })
}

fn list_to_json(items: &[String]) -> String {
    serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string())
}

/// A malformed list reads as empty rather than hiding the whole manga.
fn list_from_json(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn status_to_str(s: MangaStatus) -> &'static str {
    match s {
        MangaStatus::Ongoing => "ongoing",
        MangaStatus::Completed => "completed",
        MangaStatus::Hiatus => "hiatus",
        MangaStatus::Cancelled => "cancelled",
        MangaStatus::Unknown => "unknown",
    }
}

fn status_from_str(s: &str) -> MangaStatus {
    match s {
        "ongoing" => MangaStatus::Ongoing,
        "completed" => MangaStatus::Completed,
        "hiatus" => MangaStatus::Hiatus,
        "cancelled" => MangaStatus::Cancelled,
        _ => MangaStatus::Unknown,
    }
}
