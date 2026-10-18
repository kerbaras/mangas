//! Raw table rows, to compare a library before and after a round trip.

use std::path::Path;

use rusqlite::types::Value;
use rusqlite::Connection;

pub fn rows(conn: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut stmt = conn.prepare(sql).unwrap();
    let width = stmt.column_count();
    stmt.query_map([], |r| (0..width).map(|i| r.get(i)).collect())
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn columns_except(conn: &Connection, table: &str, skip: &[&str]) -> String {
    let names = rows(
        conn,
        &format!("SELECT name FROM pragma_table_info('{table}')"),
    );
    let names: Vec<String> = names
        .into_iter()
        .filter_map(|r| match &r[0] {
            Value::Text(n) if !skip.contains(&n.as_str()) => Some(n.clone()),
            _ => None,
        })
        .collect();
    names.join(", ")
}

/// Every library row a backup is responsible for. `SELECT *` on purpose: a column added by a
/// later migration fails the round trip until the backup carries it (or it is skipped here).
pub fn snapshot(db: &Path) -> Vec<Vec<Vec<Value>>> {
    let conn = Connection::open(db).unwrap();
    let chapter = columns_except(&conn, "chapter", &["local_path"]);
    let tables = rows(
        &conn,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    );
    let tables: Vec<_> = tables.into_iter().map(|r| r[0].clone()).collect();
    let known = [
        "category",
        "chapter",
        "download_jobs",
        "download_queue",
        // Paths to EPUBs on this machine: machine-local like the queue, not in backups.
        "export_history",
        "manga",
        "manga_category",
        "manga_language_sync",
        "setting",
    ];
    let known: Vec<_> = known.iter().map(|t| Value::Text(t.to_string())).collect();
    assert_eq!(tables, known, "new table: decide whether backups carry it");
    vec![
        rows(&conn, "SELECT * FROM manga ORDER BY rowid"),
        rows(
            &conn,
            "SELECT name, position FROM category ORDER BY position",
        ),
        rows(
            &conn,
            "SELECT mc.source, mc.manga_id, c.name FROM manga_category mc
             JOIN category c ON c.id = mc.category_id ORDER BY 1, 2, 3",
        ),
        rows(&conn, "SELECT * FROM manga_language_sync ORDER BY 1, 2, 3"),
        rows(
            &conn,
            &format!("SELECT {chapter} FROM chapter ORDER BY rowid"),
        ),
    ]
}
