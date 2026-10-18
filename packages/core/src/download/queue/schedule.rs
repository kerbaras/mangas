//! Which queued jobs start now. Pure: the counts include jobs running in any process.

use std::collections::{HashMap, HashSet};

use super::job::{Job, JobId, JobState};

/// Concurrent chapter downloads, per provider and overall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub per_provider: usize,
    pub global: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            per_provider: 2,
            global: 4,
        }
    }
}

impl Limits {
    /// `n` chapters at once ([`crate::library::Settings::concurrency`]), at most half of them
    /// from one provider but never fewer than two, so 4 gives the default 2 / 4.
    pub fn for_concurrency(n: u8) -> Self {
        let global = usize::from(n.max(1));
        Limits {
            per_provider: global.div_ceil(2).max(2).min(global),
            global,
        }
    }
}

/// The jobs one worker may claim: every job, or only the ones it queued (the CLI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    All,
    Jobs(HashSet<JobId>),
}

impl Scope {
    pub fn contains(&self, id: JobId) -> bool {
        match self {
            Scope::All => true,
            Scope::Jobs(ids) => ids.contains(&id),
        }
    }
}

/// Queue order: first queued first.
pub fn in_queue_order(mut jobs: Vec<Job>) -> Vec<Job> {
    jobs.sort_by_key(|j| (j.created_at, j.id));
    jobs
}

/// Queued jobs in `scope` to start at `now`, in queue order, within `limits`. A job waiting
/// out a retry backoff is skipped and does not block the ones behind it. Two releases of the
/// same chapter number never run together: they pick their directory from what is on disk.
pub fn pick(jobs: &[Job], limits: Limits, scope: &Scope, now: i64) -> Vec<JobId> {
    let running: Vec<&Job> = jobs
        .iter()
        .filter(|j| j.state == JobState::Running)
        .collect();
    let mut total = running.len();
    let mut per: HashMap<&str, usize> = HashMap::new();
    for job in &running {
        *per.entry(job.source.as_str()).or_default() += 1;
    }
    let mut queued: Vec<&Job> = jobs
        .iter()
        .filter(|j| j.state == JobState::Queued && j.not_before <= now && scope.contains(j.id))
        .collect();
    queued.sort_by_key(|j| (j.created_at, j.id));
    let mut busy: Vec<(&str, &str, &str)> = running.iter().map(|j| chapter_key(j)).collect();
    let mut picked = Vec::new();
    for job in queued {
        if total >= limits.global {
            break;
        }
        let count = per.entry(job.source.as_str()).or_default();
        if *count < limits.per_provider && !busy.contains(&chapter_key(job)) {
            *count += 1;
            total += 1;
            busy.push(chapter_key(job));
            picked.push(job.id);
        }
    }
    picked
}

/// The chapter directory a job writes to is keyed like this (an unnumbered one uses its id).
fn chapter_key(job: &Job) -> (&str, &str, &str) {
    let name = if job.number.is_empty() {
        &job.chapter_id.0
    } else {
        &job.number
    };
    (&job.source, &job.manga_id.0, name)
}

/// No job in `scope` is queued (retry backoff included) or running in any process.
pub fn is_settled(jobs: &[Job], scope: &Scope) -> bool {
    !jobs
        .iter()
        .any(|j| scope.contains(j.id) && matches!(j.state, JobState::Queued | JobState::Running))
}
