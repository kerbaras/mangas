use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::domain::{Error, Result};

const MIGRATIONS: &[&str] = &[
    include_str!("../../migrations/0001_init.sql"),
    include_str!("../../migrations/0002_settings.sql"),
];

/// Handle to the library database. The connection sits behind a mutex so a handle can be shared
/// across async tasks (downloads mark chapters while the UI lists them).
pub struct Library {
    conn: Mutex<Connection>,
}

impl Library {
    pub(crate) fn conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| Error::Other("library connection poisoned".into()))
    }
}

/// Opens (or creates) the library at `path` and applies pending migrations.
pub fn open(path: &Path) -> Result<Library> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
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
    Ok(Library {
        conn: Mutex::new(conn),
    })
}
