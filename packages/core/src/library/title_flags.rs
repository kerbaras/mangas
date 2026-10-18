//! Per-title flags set from the details view: when the user last opened it (clears the "new"
//! badge), whether update checks queue its new chapters for download, and the scanlation group
//! its chapter list is narrowed to.

use rusqlite::{params, OptionalExtension};

use super::Library;
use crate::domain::{normalize_group, MangaId, Result};

/// Records that the user just looked at the title, so its chapters stop counting as new.
/// Returns false when the title is not followed.
pub fn mark_viewed(lib: &Library, source: &str, id: &MangaId) -> Result<bool> {
    let changed = lib.conn()?.execute(
        "UPDATE manga SET last_viewed_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
         WHERE source = ?1 AND id = ?2",
        params![source, id.0],
    )?;
    Ok(changed > 0)
}

/// Turns "download new chapters by themselves" on or off for a followed title. Returns false
/// when the title is not followed.
pub fn set_auto_download(lib: &Library, source: &str, id: &MangaId, on: bool) -> Result<bool> {
    let changed = lib.conn()?.execute(
        "UPDATE manga SET auto_download = ?3 WHERE source = ?1 AND id = ?2",
        params![source, id.0, on],
    )?;
    Ok(changed > 0)
}

/// Narrows a followed title to one scanlation group (`None` or blank: every group). Returns
/// false when the title is not followed.
pub fn set_chapter_group(
    lib: &Library,
    source: &str,
    id: &MangaId,
    group: Option<&str>,
) -> Result<bool> {
    let changed = lib.conn()?.execute(
        "UPDATE manga SET chapter_group = ?3 WHERE source = ?1 AND id = ?2",
        params![source, id.0, normalize_group(group)],
    )?;
    Ok(changed > 0)
}

/// The group a title is narrowed to; `None` for every group or a title not followed.
pub fn chapter_group(lib: &Library, source: &str, id: &MangaId) -> Result<Option<String>> {
    let group = lib
        .conn()?
        .query_row(
            "SELECT chapter_group FROM manga WHERE source = ?1 AND id = ?2",
            params![source, id.0],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?;
    Ok(group.flatten())
}
