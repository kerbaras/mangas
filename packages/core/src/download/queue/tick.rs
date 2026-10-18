//! One scheduling step of a worker as a pure function: given every job and what this worker
//! saw since the last step, the jobs to save and the ones it now claims.

use std::collections::HashMap;
use std::time::Duration;

use super::job::{Job, JobId};
use super::lease::{mark_stale, LEASE_MS};
use super::retry::{Failure, RetryPolicy};
use super::schedule::{pick, Limits, Scope};
use super::transitions::{claim, fail, finish, heartbeat, is_run_by, recover};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueConfig {
    pub limits: Limits,
    pub retry: RetryPolicy,
    /// Pause between two steps when nothing wakes the worker earlier.
    pub tick: Duration,
    /// A running job whose heartbeat is older than this belongs to a process that is gone
    /// ([`LEASE_MS`] by default; the worker renews every `tick`).
    pub stale_after_ms: i64,
}

impl Default for QueueConfig {
    fn default() -> Self {
        QueueConfig {
            limits: Limits::default(),
            retry: RetryPolicy::default(),
            tick: Duration::from_millis(500),
            stale_after_ms: LEASE_MS,
        }
    }
}

/// What the worker knows besides the database.
pub struct TickInput<'a> {
    pub owner: &'a str,
    pub now: i64,
    /// Jobs this worker is downloading or just finished, with their `(pages done, total)` once
    /// known.
    pub in_flight: &'a HashMap<JobId, Option<(usize, usize)>>,
    /// Jobs whose task ended since the last step: `None` for success.
    pub finished: &'a [(JobId, Option<Failure>)],
    pub scope: &'a Scope,
    pub config: &'a QueueConfig,
    /// The queue-wide pause: running jobs carry on, nothing new is claimed.
    pub paused: bool,
}

#[derive(Debug, Default)]
pub struct TickPlan {
    /// Every job after the step, in queue order.
    pub jobs: Vec<Job>,
    /// The jobs that differ from the database and must be saved.
    pub changed: Vec<Job>,
    /// Jobs this worker claimed and must start.
    pub claimed: Vec<Job>,
}

pub fn plan_tick(jobs: Vec<Job>, input: &TickInput<'_>) -> TickPlan {
    let stepped: Vec<(Job, Option<Job>)> = jobs
        .into_iter()
        .map(|job| {
            let next = step(&job, input);
            (job, next)
        })
        .collect();
    let mut changed: Vec<Job> = stepped.iter().filter_map(|(_, n)| n.clone()).collect();
    let mut all: Vec<Job> = stepped
        .into_iter()
        .map(|(job, next)| next.unwrap_or(job))
        .collect();
    let picks = match input.paused {
        true => Vec::new(),
        false => pick(&all, input.config.limits, input.scope, input.now),
    };
    let mut claimed = Vec::new();
    for job in all.iter_mut().filter(|j| picks.contains(&j.id)) {
        *job = claim(job, input.owner, input.now);
        changed.retain(|c| c.id != job.id);
        changed.push(job.clone());
        claimed.push(job.clone());
    }
    TickPlan {
        jobs: mark_stale(all, input.now, input.config.stale_after_ms),
        changed,
        claimed,
    }
}

fn step(job: &Job, input: &TickInput<'_>) -> Option<Job> {
    let (owner, now) = (input.owner, input.now);
    if let Some((_, outcome)) = input.finished.iter().find(|(id, _)| *id == job.id) {
        // A chapter can finish between two heartbeats: keep its last page count.
        let job = &heartbeat(job, input.in_flight.get(&job.id).copied().flatten(), now);
        return match outcome {
            None => finish(job, owner, now),
            Some(failure) => fail(job, owner, failure, &input.config.retry, now),
        };
    }
    if is_run_by(job, owner) {
        return match input.in_flight.get(&job.id) {
            Some(pages) => Some(heartbeat(job, *pages, now)),
            // Ours, yet no task runs it (aborted, or a lost result): queue it again now.
            None => recover(job, now, -1),
        };
    }
    recover(job, now, input.config.stale_after_ms)
}
