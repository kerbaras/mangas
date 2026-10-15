use rusqlite::{params, Row};

use super::Library;
use crate::domain::{Manga, MangaId, MangaStatus, Result};

/// Inserts the manga or refreshes its metadata if it is already followed.
pub fn upsert_manga(lib: &Library, manga: &Manga) -> Result<()> {
    lib.conn.execute(
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

/// All followed manga, most recently added first.
pub fn list_manga(lib: &Library) -> Result<Vec<Manga>> {
    let mut stmt = lib.conn.prepare(
        "SELECT id, source, title, description, cover_url, status
         FROM manga ORDER BY added_at DESC, title",
    )?;
    let rows = stmt.query_map([], row_to_manga)?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
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
