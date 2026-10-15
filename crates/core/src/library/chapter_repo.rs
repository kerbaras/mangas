use std::path::Path;

use rusqlite::{params, Row};

use super::Library;
use crate::domain::{sort_by_number, Chapter, ChapterId, MangaId, Result};

/// Inserts or refreshes chapter metadata. `local_path` is left untouched so a re-sync never
/// forgets what is already on disk. The manga must be followed (foreign key).
pub fn upsert_chapters(lib: &Library, source: &str, chapters: &[Chapter]) -> Result<()> {
    let mut conn = lib.conn()?;
    let tx = conn.transaction()?;
    {
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
    }
    tx.commit()?;
    Ok(())
}

/// Known chapters of a manga, by number ascending.
pub fn list_chapters(lib: &Library, source: &str, manga_id: &MangaId) -> Result<Vec<Chapter>> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare(
        "SELECT id, manga_id, title, language, volume, number, local_path
         FROM chapter WHERE source = ?1 AND manga_id = ?2",
    )?;
    let rows = stmt.query_map(params![source, manga_id.0], row_to_chapter)?;
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
