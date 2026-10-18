//! `download_jobs` rows. Every write runs in an IMMEDIATE transaction so the desktop app and
//! the CLI, sharing the file, never interleave a read-modify-write of the queue.

use rusqlite::{params, Connection, Row, TransactionBehavior};

use super::Library;
use crate::domain::{ChapterId, MangaId, Result};
use crate::download::queue::{enqueue_job, Job, JobId, JobState};

const SELECT: &str = "SELECT j.id, j.source, c.manga_id, j.chapter_id, j.state, j.attempts,
        j.error, j.pages_done, j.pages_total, j.not_before, j.owner, j.heartbeat_at,
        j.created_at, j.updated_at, COALESCE(m.title, ''), c.number, c.title, c.language,
        c.local_path
    FROM download_jobs j
    JOIN chapter c ON c.source = j.source AND c.id = j.chapter_id
    LEFT JOIN manga m ON m.source = c.source AND m.id = c.manga_id
    ORDER BY j.created_at, j.id";

/// Every job in queue order, with its chapter's display fields.
pub fn list_jobs(lib: &Library) -> Result<Vec<Job>> {
    let conn = lib.conn()?;
    load(&conn)
}

/// Loads every job, lets `f` decide which to save (returned with their new values), saves
/// them, and returns `f`'s result, all in one transaction.
pub fn update_jobs<T>(lib: &Library, f: impl FnOnce(Vec<Job>) -> (Vec<Job>, T)) -> Result<T> {
    let mut conn = lib.conn()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (changed, out) = f(load(&tx)?);
    for job in &changed {
        save(&tx, job)?;
    }
    tx.commit()?;
    Ok(out)
}

/// Queues the chapters `chapters` of `source` (each must be in the library). A chapter whose
/// job is still active keeps it; any other is (re)queued at the back. Returns the job ids.
pub fn enqueue_jobs(
    lib: &Library,
    source: &str,
    chapters: &[ChapterId],
    now: i64,
) -> Result<Vec<JobId>> {
    let mut conn = lib.conn()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let jobs = load(&tx)?;
    let mut ids = Vec::new();
    for chapter in chapters {
        let existing = jobs
            .iter()
            .find(|j| j.source == source && &j.chapter_id == chapter);
        let fresh = Job {
            source: source.to_string(),
            chapter_id: chapter.clone(),
            ..Job::default()
        };
        match (existing, enqueue_job(existing, fresh, now)) {
            (Some(job), Some(next)) => {
                save(&tx, &next)?;
                ids.push(job.id);
            }
            (Some(job), None) => ids.push(job.id),
            (None, Some(next)) => {
                tx.execute(
                    "INSERT INTO download_jobs (source, chapter_id, state, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![source, chapter.0, next.state.as_str(), now],
                )?;
                ids.push(tx.last_insert_rowid());
            }
            (None, None) => {}
        }
    }
    tx.commit()?;
    Ok(ids)
}

/// Deletes the jobs in `states`; returns how many.
pub fn delete_jobs(lib: &Library, states: &[JobState]) -> Result<usize> {
    let conn = lib.conn()?;
    let mut removed = 0;
    for state in states {
        removed += conn.execute(
            "DELETE FROM download_jobs WHERE state = ?1",
            params![state.as_str()],
        )?;
    }
    Ok(removed)
}

fn load(conn: &Connection) -> Result<Vec<Job>> {
    let mut stmt = conn.prepare(SELECT)?;
    let rows = stmt.query_map([], row_to_job)?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

fn save(conn: &Connection, job: &Job) -> Result<()> {
    conn.execute(
        "UPDATE download_jobs SET state = ?2, attempts = ?3, error = ?4, pages_done = ?5,
            pages_total = ?6, not_before = ?7, owner = ?8, heartbeat_at = ?9, created_at = ?10,
            updated_at = ?11
         WHERE id = ?1",
        params![
            job.id,
            job.state.as_str(),
            job.attempts,
            job.error,
            job.pages_done as i64,
            job.pages_total as i64,
            job.not_before,
            job.owner,
            job.heartbeat_at,
            job.created_at,
            job.updated_at,
        ],
    )?;
    Ok(())
}

fn row_to_job(row: &Row<'_>) -> rusqlite::Result<Job> {
    let state: String = row.get(4)?;
    Ok(Job {
        id: row.get(0)?,
        source: row.get(1)?,
        manga_id: MangaId(row.get(2)?),
        chapter_id: ChapterId(row.get(3)?),
        // An unknown state (a newer app wrote it) is treated as paused: kept, never run.
        state: JobState::parse(&state).unwrap_or(JobState::Paused),
        attempts: row.get(5)?,
        error: row.get(6)?,
        pages_done: row.get::<_, i64>(7)?.max(0) as usize,
        pages_total: row.get::<_, i64>(8)?.max(0) as usize,
        not_before: row.get(9)?,
        owner: row.get(10)?,
        heartbeat_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        manga_title: row.get(14)?,
        number: row.get(15)?,
        title: row.get(16)?,
        language: row.get(17)?,
        local_path: row.get(18)?,
    })
}
