//! Where a data-folder move is copying to, recorded for as long as the copy can be left behind
//! (a killed move), so the partial copy can be found again.

use rusqlite::{params, OptionalExtension};

use super::Library;
use crate::domain::Result;

const KEY: &str = "pending_move";

/// The target folder of a move that started and didn't finish (or is still running).
pub fn pending_move(lib: &Library) -> Result<Option<String>> {
    let sql = "SELECT value FROM setting WHERE key = ?1";
    Ok(lib.conn()?.query_row(sql, [KEY], |r| r.get(0)).optional()?)
}

/// Records the move's target folder, or clears the record with `None`.
pub fn set_pending_move(lib: &Library, target: Option<&str>) -> Result<()> {
    let conn = lib.conn()?;
    match target {
        Some(target) => conn.execute(
            "INSERT INTO setting (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![KEY, target],
        )?,
        None => conn.execute("DELETE FROM setting WHERE key = ?1", [KEY])?,
    };
    Ok(())
}
