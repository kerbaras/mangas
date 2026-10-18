use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::domain::{Error, Result};

const MIGRATIONS: &[&str] = &[
    include_str!("../../migrations/0001_init.sql"),
    include_str!("../../migrations/0002_settings.sql"),
    include_str!("../../migrations/0003_chapter_group.sql"),
    include_str!("../../migrations/0004_chapter_availability.sql"),
    include_str!("../../migrations/0005_manga_metadata.sql"),
    include_str!("../../migrations/0006_library_organisation.sql"),
    include_str!("../../migrations/0007_download_jobs.sql"),
    include_str!("../../migrations/0008_update_checks.sql"),
    include_str!("../../migrations/0009_job_force.sql"),
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

fn init(mut conn: Connection) -> Result<Library> {
    // The desktop app and the CLI share the file: wait for the other's write instead of failing.
    conn.execute_batch(
        "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;",
    )?;
    migrate(&mut conn, MIGRATIONS)?;
    Ok(Library {
        conn: Mutex::new(conn),
    })
}

/// Each migration commits together with its version bump, so an interrupted one reruns whole.
fn migrate(conn: &mut Connection, migrations: &[&str]) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (i, sql) in migrations.iter().enumerate().skip(version as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as i64 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn a_failed_migration_is_rolled_back_and_retried_whole() {
        let mut conn = Connection::open_in_memory().unwrap();
        let create = "CREATE TABLE t (a INTEGER);";
        let broken = "ALTER TABLE t ADD COLUMN b TEXT; ALTER TABLE missing ADD COLUMN c TEXT;";
        assert!(migrate(&mut conn, &[create, broken]).is_err());
        assert_eq!(version(&conn), 1);

        migrate(&mut conn, &[create, "ALTER TABLE t ADD COLUMN b TEXT;"]).unwrap();
        assert_eq!(version(&conn), 2);
    }

    /// Parallel branches that both add `000N_*.sql` collide here instead of one silently never running.
    #[test]
    fn migration_files_are_numbered_once_each_and_all_registered_in_order() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let mut files: Vec<(u32, String)> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.ends_with(".sql"))
            .map(|name| (name[..4].parse().unwrap(), name))
            .collect();
        files.sort();

        let numbers: Vec<u32> = files.iter().map(|(n, _)| *n).collect();
        let expected: Vec<u32> = (1..=files.len() as u32).collect();
        assert_eq!(
            numbers, expected,
            "migration numbers must be unique and consecutive: {files:?}"
        );
        assert_eq!(
            MIGRATIONS.len(),
            files.len(),
            "every migration file is in MIGRATIONS"
        );
        for ((_, name), sql) in files.iter().zip(MIGRATIONS) {
            assert_eq!(
                std::fs::read_to_string(dir.join(name)).unwrap(),
                *sql,
                "{name} out of order"
            );
        }
    }
}
