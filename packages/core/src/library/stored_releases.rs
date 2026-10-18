//! The releases a title already has, for telling another group's release from a new chapter.

use rusqlite::{params, Connection};

use crate::domain::{Availability, Chapter, MangaId, Result};

/// Language, volume, number and availability of a title's stored chapters, narrowed to one
/// language and/or one scanlation group when given.
pub(crate) fn stored_releases(
    conn: &Connection,
    source: &str,
    manga_id: &MangaId,
    language: Option<&str>,
    group: Option<&str>,
) -> Result<Vec<Chapter>> {
    let mut stmt = conn.prepare(
        "SELECT language, volume, number, availability FROM chapter
         WHERE source = ?1 AND manga_id = ?2 AND (?3 IS NULL OR language = ?3)
           AND (?4 IS NULL OR scanlation_group = ?4)",
    )?;
    let rows = stmt.query_map(params![source, manga_id.0, language, group], |r| {
        Ok(Chapter {
            language: r.get(0)?,
            volume: r.get(1)?,
            number: r.get(2)?,
            availability: Availability::from_columns(&r.get::<_, String>(3)?, None),
            ..Chapter::default()
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}
