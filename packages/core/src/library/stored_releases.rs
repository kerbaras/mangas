//! The releases a title already has, for telling another group's release from a new chapter.

use rusqlite::{params, Connection, OptionalExtension};

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

/// What a fetched release is checked against to tell a new chapter from another group's release
/// of a stored one: every group's releases, or only the saved group's (#160) when the release is
/// that group's, so its own release of a chapter another group had still counts as new (#167).
pub(crate) struct KnownReleases {
    all: Vec<Chapter>,
    saved: Option<(String, Vec<Chapter>)>,
}

impl KnownReleases {
    pub(crate) fn load(
        conn: &Connection,
        source: &str,
        manga_id: &MangaId,
        language: &str,
    ) -> Result<Self> {
        let all = stored_releases(conn, source, manga_id, Some(language), None)?;
        let saved = match saved_group(conn, source, manga_id)? {
            Some(group) => {
                let releases =
                    stored_releases(conn, source, manga_id, Some(language), Some(&group))?;
                Some((group, releases))
            }
            None => None,
        };
        Ok(Self { all, saved })
    }

    pub(crate) fn for_release(&self, release: &Chapter) -> &[Chapter] {
        match &self.saved {
            Some((group, releases)) if release.group.as_deref() == Some(group) => releases,
            _ => &self.all,
        }
    }
}

/// The group a title is narrowed to (#160), `None` for all groups.
fn saved_group(conn: &Connection, source: &str, manga_id: &MangaId) -> Result<Option<String>> {
    let group = conn
        .query_row(
            "SELECT chapter_group FROM manga WHERE source = ?1 AND id = ?2",
            params![source, manga_id.0],
            |r| r.get(0),
        )
        .optional()?;
    Ok(group.flatten())
}
