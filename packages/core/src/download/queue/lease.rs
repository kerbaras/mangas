//! A running job's lease: its owner renews `heartbeat_at` on every worker tick, so a job whose
//! heartbeat is older than the lease belongs to a process that died (kill -9, crash) and any
//! worker may take it back.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::job::{Job, JobState};
use super::ops::now_ms;

/// How long a running job's lease lasts without renewal: 30 worker ticks (500 ms each).
pub const LEASE_MS: i64 = 15_000;

/// Unix ms "now" for lease decisions; tests pass their own to move time without sleeping.
pub type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

/// Running, but its owner stopped renewing the lease more than `stale_after` ms ago.
pub fn lease_expired(job: &Job, now: i64, stale_after: i64) -> bool {
    job.state == JobState::Running && now - job.heartbeat_at > stale_after
}

/// Running under a live lease: some process is downloading it right now.
pub fn holds_lease(job: &Job, now: i64, stale_after: i64) -> bool {
    job.state == JobState::Running && !lease_expired(job, now, stale_after)
}

/// `jobs` with [`Job::stale_lease`] set on the running ones whose lease ran out.
pub fn mark_stale(jobs: Vec<Job>, now: i64, stale_after: i64) -> Vec<Job> {
    jobs.into_iter()
        .map(|job| Job {
            stale_lease: lease_expired(&job, now, stale_after),
            ..job
        })
        .collect()
}

/// A lease owner name unique to this worker: process id, start time, and a counter for two
/// workers of one process created in the same millisecond.
pub fn new_owner() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}-{n}", std::process::id(), now_ms())
}
