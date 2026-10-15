use rusqlite::{params, OptionalExtension, Row};

use super::Library;
use crate::domain::{Manga, MangaId, MangaStatus, Result};

const COLUMNS: &str = "id, source, title, description, cover_url, status";

/// Inserts the manga or refreshes its metadata if it is already followed.
pub fn upsert_manga(lib: &Library, manga: &Manga) -> Result<()> {
    lib.conn()?.execute(
        "INSERT INTO manga (id, source, title, description, cover_url, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (source, id) DO UPDATE SET
            title = excluded.title, description = excluded.description,
            cover_url = excluded.cover_url, status = excluded.status",
        params![
            manga.id.0,
            manga.source,
            manga.title,
            manga.description,
            manga.cover_url,
            status_to_str(manga.status),
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

fn row_to_manga(row: &Row<'_>) -> rusqlite::Result<Manga> {
    Ok(Manga {
        id: MangaId(row.get(0)?),
        source: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        cover_url: row.get(4)?,
        status: status_from_str(&row.get::<_, String>(5)?),
    })
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
