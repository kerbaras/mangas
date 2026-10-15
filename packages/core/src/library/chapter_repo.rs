use std::path::Path;

use rusqlite::{params, Row};

use super::Library;
use crate::domain::{sort_by_number, Chapter, ChapterId, MangaId, Result};

/// Stores the `language` chapter list of `manga_id` as the source currently reports it.
/// Metadata is refreshed, `local_path` is left untouched so a re-sync never forgets what is
/// on disk, and chapters the source no longer lists are dropped unless downloaded. An empty
/// `chapters` therefore clears every non-downloaded chapter of that language.
/// The manga must be followed.
pub fn upsert_chapters(
    lib: &Library,
    source: &str,
    manga_id: &MangaId,
    language: &str,
    chapters: &[Chapter],
) -> Result<()> {
    let mut conn = lib.conn()?;
    let tx = conn.transaction()?;
    tx.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS synced (id TEXT PRIMARY KEY); DELETE FROM synced;",
    )?;
    {
        let mut seen = tx.prepare("INSERT OR IGNORE INTO synced (id) VALUES (?1)")?;
        for c in chapters {
            seen.execute(params![c.id.0])?;
        }
        let mut stmt = tx.prepare(
            "INSERT INTO chapter (id, manga_id, source, title, language, volume, number)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (source, id) DO UPDATE SET
                title = excluded.title, language = excluded.language,
                volume = excluded.volume, number = excluded.number",
        )?;
        for c in chapters {
            stmt.execute(params![
                c.id.0,
                c.manga_id.0,
                source,
                c.title,
                c.language,
                c.volume,
                c.number,
            ])?;
        }
        tx.execute(
            "DELETE FROM chapter
             WHERE source = ?1 AND manga_id = ?2 AND language = ?3 AND local_path IS NULL
               AND id NOT IN (SELECT id FROM synced)",
            params![source, manga_id.0, language],
        )?;
    }
    tx.execute_batch("DROP TABLE synced;")?;
    tx.commit()?;
    Ok(())
}

/// Known chapters of a manga, by number ascending; `language` narrows to one language.
pub fn list_chapters(
    lib: &Library,
    source: &str,
    manga_id: &MangaId,
    language: Option<&str>,
) -> Result<Vec<Chapter>> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare(
        "SELECT id, manga_id, title, language, volume, number, local_path
         FROM chapter WHERE source = ?1 AND manga_id = ?2 AND (?3 IS NULL OR language = ?3)",
    )?;
    let rows = stmt.query_map(params![source, manga_id.0, language], row_to_chapter)?;
    let chapters: Vec<Chapter> = rows.collect::<std::result::Result<_, _>>()?;
    Ok(sort_by_number(chapters))
}

/// Records where a chapter's pages live on disk.
pub fn mark_downloaded(
    lib: &Library,
    source: &str,
    chapter: &ChapterId,
    path: &Path,
) -> Result<()> {
    lib.conn()?.execute(
        "UPDATE chapter SET local_path = ?3 WHERE source = ?1 AND id = ?2",
        params![source, chapter.0, path.to_string_lossy()],
    )?;
    Ok(())
}

fn row_to_chapter(row: &Row<'_>) -> rusqlite::Result<Chapter> {
    Ok(Chapter {
        id: ChapterId(row.get(0)?),
        manga_id: MangaId(row.get(1)?),
        title: row.get(2)?,
        language: row.get(3)?,
        volume: row.get(4)?,
        number: row.get(5)?,
        local_path: row.get(6)?,
    })
}
