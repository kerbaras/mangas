use std::path::Path;

use rusqlite::{params, Row};

use super::Library;
use crate::domain::{sort_by_number, Availability, Chapter, ChapterId, MangaId, Result};
use crate::download::queue::{now_ms, UNLISTED};

/// Stores the `language` chapter list of `manga_id` as the source currently reports it.
/// Metadata is refreshed, `local_path` is left untouched so a re-sync never forgets what is
/// on disk, and chapters the source no longer lists are dropped unless downloaded or still in
/// the download queue (their job fails with [`UNLISTED`]). An empty
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
        // A chapter the library hasn't seen yet makes the title "updated".
        tx.execute(
            "UPDATE manga SET updated_at = datetime('now')
             WHERE source = ?1 AND id = ?2 AND EXISTS (
                SELECT 1 FROM synced s WHERE NOT EXISTS (
                    SELECT 1 FROM chapter c WHERE c.source = ?1 AND c.id = s.id))",
            params![source, manga_id.0],
        )?;
        let mut stmt = tx.prepare(
            "INSERT INTO chapter
                (id, manga_id, source, title, language, volume, number, scanlation_group,
                 availability, external_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT (source, id) DO UPDATE SET
                title = excluded.title, language = excluded.language,
                volume = excluded.volume, number = excluded.number,
                scanlation_group = excluded.scanlation_group,
                availability = excluded.availability, external_url = excluded.external_url",
        )?;
        for c in chapters {
            let (availability, external_url) = c.availability.to_columns();
            stmt.execute(params![
                c.id.0,
                c.manga_id.0,
                source,
                c.title,
                c.language,
                c.volume,
                c.number,
                c.group,
                availability,
                external_url,
            ])?;
        }
        // A dropped chapter that still has a download on its way keeps its row: the job fails
        // with the reason instead of vanishing with it, and stays until the user clears it.
        tx.execute(
            "UPDATE download_jobs
             SET state = 'failed', error = ?4, owner = NULL, not_before = 0, updated_at = ?5
             WHERE source = ?1 AND state IN ('queued', 'running', 'paused')
               AND chapter_id IN (SELECT id FROM chapter
                   WHERE source = ?1 AND manga_id = ?2 AND language = ?3 AND local_path IS NULL
                     AND id NOT IN (SELECT id FROM synced))",
            params![source, manga_id.0, language, UNLISTED, now_ms()],
        )?;
        tx.execute(
            "DELETE FROM chapter
             WHERE source = ?1 AND manga_id = ?2 AND language = ?3 AND local_path IS NULL
               AND id NOT IN (SELECT id FROM synced)
               AND id NOT IN (SELECT chapter_id FROM download_jobs
                   WHERE source = ?1 AND state IN ('queued', 'running', 'paused', 'failed'))",
            params![source, manga_id.0, language],
        )?;
    }
    tx.execute_batch("DROP TABLE synced;")?;
    tx.commit()?;
    Ok(())
}

/// Known chapters of a manga, by number ascending. `language` narrows to one language, but
/// downloaded chapters are always listed (with their own language) so a preference change never
/// hides what is on disk.
pub fn list_chapters(
    lib: &Library,
    source: &str,
    manga_id: &MangaId,
    language: Option<&str>,
) -> Result<Vec<Chapter>> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare(
        "SELECT id, manga_id, title, language, volume, number, local_path, scanlation_group,
                availability, external_url,
                strftime('%Y-%m-%dT%H:%M:%SZ', exported_at)
         FROM chapter WHERE source = ?1 AND manga_id = ?2
               AND (?3 IS NULL OR language = ?3 OR local_path IS NOT NULL)",
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

/// Records that the chapters downloaded to `dirs` were just exported. Paths match the stored
/// `local_path` ignoring a trailing separator; folders that aren't library chapters are skipped.
/// Returns how many chapter rows were marked.
pub fn mark_exported(lib: &Library, dirs: &[&Path]) -> Result<usize> {
    let mut conn = lib.conn()?;
    let tx = conn.transaction()?;
    let mut marked = 0;
    {
        let mut stmt = tx.prepare(
            "UPDATE chapter SET exported_at = datetime('now')
             WHERE local_path IS NOT NULL AND rtrim(local_path, '/\\') = ?1",
        )?;
        for dir in dirs {
            let path = dir.to_string_lossy();
            marked += stmt.execute(params![path.trim_end_matches(['/', '\\'])])?;
        }
    }
    tx.commit()?;
    Ok(marked)
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
        group: row.get(7)?,
        availability: Availability::from_columns(&row.get::<_, String>(8)?, row.get(9)?),
        exported_at: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Manga;
    use crate::library::{list_entries, open_in_memory, upsert_manga};

    fn chapter(id: &str) -> Chapter {
        Chapter {
            id: ChapterId(id.into()),
            manga_id: MangaId("a".into()),
            language: "en".into(),
            number: id.trim_start_matches('c').into(),
            ..Default::default()
        }
    }

    #[test]
    fn only_a_chapter_the_library_did_not_have_moves_updated_at() {
        let lib = open_in_memory().unwrap();
        let id = MangaId("a".into());
        let manga = Manga {
            id: id.clone(),
            source: "s".into(),
            ..Default::default()
        };
        upsert_manga(&lib, &manga).unwrap();
        let entry = list_entries(&lib).unwrap().remove(0);
        assert_eq!(entry.updated_at, entry.added_at);
        let old = "2020-01-01T00:00:00Z";
        let sync = |list: &[Chapter]| {
            let backdate = "UPDATE manga SET updated_at = '2020-01-01 00:00:00'";
            lib.conn().unwrap().execute(backdate, []).unwrap();
            upsert_chapters(&lib, "s", &id, "en", list).unwrap();
            list_entries(&lib).unwrap().remove(0).updated_at
        };
        assert_ne!(sync(&[chapter("c1")]), old, "first chapters are new");
        assert_eq!(sync(&[chapter("c1")]), old, "nothing new");
        assert_ne!(sync(&[chapter("c1"), chapter("c2")]), old);
    }
}
