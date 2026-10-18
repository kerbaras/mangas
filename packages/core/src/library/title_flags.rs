//! Per-title flags set from the details view: when the user last opened it (clears the "new"
//! badge) and whether update checks queue its new chapters for download.

use rusqlite::params;

use super::Library;
use crate::domain::{MangaId, Result};

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
