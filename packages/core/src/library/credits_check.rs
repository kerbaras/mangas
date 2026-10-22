//! Whether the update check already re-fetched an uncredited title's record.

use rusqlite::{params, OptionalExtension};

use super::Library;
use crate::domain::{MangaId, Result};

/// True once [`mark_credits_checked`] ran for the title (an unknown title is not checked).
pub fn credits_checked(lib: &Library, source: &str, id: &MangaId) -> Result<bool> {
    let at: Option<Option<String>> = lib
        .conn()?
        .query_row(
            "SELECT credits_checked_at FROM manga WHERE source = ?1 AND id = ?2",
            params![source, id.0],
            |r| r.get(0),
        )
        .optional()?;
    Ok(matches!(at, Some(Some(_))))
}

/// Records that the title's record was re-fetched for its credits, so the check doesn't again.
pub fn mark_credits_checked(lib: &Library, source: &str, id: &MangaId) -> Result<()> {
    lib.conn()?.execute(
        "UPDATE manga SET credits_checked_at = datetime('now') WHERE source = ?1 AND id = ?2",
        params![source, id.0],
    )?;
    Ok(())
}
