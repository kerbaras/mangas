//! Per-title chapter counts folded by SQLite in one aggregate query: no chapter rows reach Rust.

use std::collections::HashMap;

use rusqlite::functions::FunctionFlags;
use rusqlite::Connection;

use super::counts::{chapter_key, ChapterCounts};
use crate::domain::Result;

/// `(source, manga id)`.
pub(super) type Key = (String, String);

/// What [`super::count_chapters`] counts, one row per title: each chapter number (its
/// `chapter_key`) once, of the saved group only when the title is narrowed to one.
const COUNTS: &str = "
    WITH keyed AS (
        SELECT c.source, c.manga_id, chapter_key(c.number, c.id) AS k,
               c.availability = 'available' AS available,
               c.local_path IS NOT NULL AS downloaded,
               c.exported_at IS NOT NULL AS exported,
               c.first_seen_at IS NOT NULL
                 AND c.first_seen_at > COALESCE(m.last_viewed_at, '') AS fresh
        FROM chapter c JOIN manga m ON m.source = c.source AND m.id = c.manga_id
        WHERE m.chapter_group IS NULL OR c.scanlation_group = m.chapter_group),
    numbers AS (
        SELECT source, manga_id, MAX(available) AS available, MAX(downloaded) AS downloaded,
               MAX(exported) AS exported, MAX(fresh) AS fresh
        FROM keyed GROUP BY source, manga_id, k)
    SELECT source, manga_id, COUNT(*), COALESCE(SUM(downloaded), 0),
           COALESCE(SUM(exported), 0), COALESCE(SUM(available AND NOT exported), 0),
           COALESCE(SUM(fresh), 0)
    FROM numbers GROUP BY source, manga_id";

fn register_chapter_key(conn: &Connection) -> rusqlite::Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC;
    conn.create_scalar_function("chapter_key", 2, flags, |ctx| {
        Ok(chapter_key(
            ctx.get_raw(0).as_str()?,
            ctx.get_raw(1).as_str()?,
        ))
    })
}

fn count(n: i64) -> usize {
    usize::try_from(n).unwrap_or(0)
}

/// Chapter counts of every title that has chapters; titles without any are absent.
pub(super) fn counts_by_manga(conn: &Connection) -> Result<HashMap<Key, ChapterCounts>> {
    register_chapter_key(conn)?;
    let mut stmt = conn.prepare(COUNTS)?;
    let rows = stmt.query_map([], |r| {
        let counts = ChapterCounts {
            total: count(r.get(2)?),
            downloaded: count(r.get(3)?),
            exported: count(r.get(4)?),
            unread: count(r.get(5)?),
            new: count(r.get(6)?),
        };
        Ok(((r.get(0)?, r.get(1)?), counts))
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

#[cfg(test)]
mod tests;
