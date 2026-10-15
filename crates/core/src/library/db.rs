use std::path::Path;

use rusqlite::Connection;

use crate::domain::Result;

const MIGRATIONS: &[&str] = &[include_str!("../../migrations/0001_init.sql")];

/// Handle to the library database. Cheap to pass around by `&`; not `Sync`, so each task
/// opens its own handle.
pub struct Library {
    pub(crate) conn: Connection,
}

/// Opens (or creates) the library at `path` and applies pending migrations.
pub fn open(path: &Path) -> Result<Library> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    init(Connection::open(path)?)
}

/// In-memory library, for tests and previews.
pub fn open_in_memory() -> Result<Library> {
    init(Connection::open_in_memory()?)
}

fn init(conn: Connection) -> Result<Library> {
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        conn.execute_batch(sql)?;
        conn.pragma_update(None, "user_version", i as i64 + 1)?;
    }
    Ok(Library { conn })
}
