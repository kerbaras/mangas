//! The queue-wide gate row (`download_queue`, migration 0007).

use rusqlite::{params, Connection};

use super::Library;
use crate::domain::Result;
use crate::download::queue::QueueGate;

pub(super) fn read_gate(conn: &Connection) -> Result<QueueGate> {
    let sql = "SELECT paused, held_until FROM download_queue WHERE id = 1";
    Ok(conn.query_row(sql, [], |r| {
        Ok(QueueGate {
            paused: r.get(0)?,
            held_until: r.get(1)?,
        })
    })?)
}

pub fn queue_gate(lib: &Library) -> Result<QueueGate> {
    let conn = lib.conn()?;
    read_gate(&conn)
}

/// The user's queue-wide pause (`queue pause --all`).
pub fn queue_paused(lib: &Library) -> Result<bool> {
    Ok(queue_gate(lib)?.paused)
}

pub fn set_queue_paused(lib: &Library, paused: bool) -> Result<()> {
    let sql = "UPDATE download_queue SET paused = ?1 WHERE id = 1";
    lib.conn()?.execute(sql, params![paused])?;
    Ok(())
}

/// Holds the queue until `until` (unix ms); 0 releases it.
pub fn set_queue_held_until(lib: &Library, until: i64) -> Result<()> {
    let sql = "UPDATE download_queue SET held_until = ?1 WHERE id = 1";
    lib.conn()?.execute(sql, params![until])?;
    Ok(())
}
