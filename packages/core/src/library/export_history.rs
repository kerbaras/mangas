//! History of exported books and their deliveries (`export_history`).

use std::path::Path;

use rusqlite::{params, Row};
use serde::Serialize;

use super::Library;
use crate::domain::Result;
use crate::send::SendVia;

/// One exported book; `sent_*` are set once it reached a device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportRecord {
    pub id: i64,
    pub file: String,
    pub title: String,
    pub device: String,
    /// UTC, `YYYY-MM-DD HH:MM:SS`; `None` for a file exported elsewhere and sent from here.
    pub exported_at: Option<String>,
    pub sent_via: Option<SendVia>,
    /// Kindle address or device name.
    pub sent_to: Option<String>,
    pub sent_at: Option<String>,
    /// Whether the file is still on disk (checked when listed).
    pub exists: bool,
}

/// Records a book just written to `file` (stored absolute, like every path in the history).
pub fn record_export(lib: &Library, file: &Path, title: &str, device: &str) -> Result<i64> {
    let file = std::path::absolute(file)?;
    let conn = lib.conn()?;
    conn.execute(
        "INSERT INTO export_history (file, title, device, exported_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
        params![file.to_string_lossy(), title, device],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Stamps the latest export of `file` as sent `via` to `to`, or adds a row when the file has
/// none (exported by another tool, or before the history existed).
pub fn record_sent(lib: &Library, file: &Path, title: &str, via: SendVia, to: &str) -> Result<()> {
    let file = std::path::absolute(file)?;
    let conn = lib.conn()?;
    let path = file.to_string_lossy();
    let updated = conn.execute(
        "UPDATE export_history SET sent_via = ?2, sent_to = ?3, sent_at = datetime('now')
         WHERE id = (SELECT max(id) FROM export_history WHERE file = ?1)",
        params![path, via.as_str(), to],
    )?;
    if updated == 0 {
        conn.execute(
            "INSERT INTO export_history (file, title, sent_via, sent_to, sent_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![path, title, via.as_str(), to],
        )?;
    }
    Ok(())
}

/// The `limit` most recent books, newest first.
pub fn list_exports(lib: &Library, limit: usize) -> Result<Vec<ExportRecord>> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare(
        "SELECT id, file, title, device, exported_at, sent_via, sent_to, sent_at
         FROM export_history ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit as i64], record)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn record(r: &Row) -> rusqlite::Result<ExportRecord> {
    let file: String = r.get(1)?;
    let via: Option<String> = r.get(5)?;
    Ok(ExportRecord {
        id: r.get(0)?,
        exists: Path::new(&file).is_file(),
        file,
        title: r.get(2)?,
        device: r.get(3)?,
        exported_at: r.get(4)?,
        sent_via: via.as_deref().and_then(SendVia::parse),
        sent_to: r.get(6)?,
        sent_at: r.get(7)?,
    })
}
