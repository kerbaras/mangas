//! Pure state transitions of one [`Job`]: data in, data out. `None` means the transition does
//! not apply to the job's current state and nothing changes.

use super::job::{Job, JobState};
use super::lease::lease_expired;
use super::retry::{backoff, Failure, RetryPolicy};
use super::UNLISTED;

/// What the user can ask of a job (or of every job at once).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Pause,
    Resume,
    Cancel,
    Retry,
}

impl Action {
    pub fn verb(self) -> &'static str {
        match self {
            Action::Pause => "pause",
            Action::Resume => "resume",
            Action::Cancel => "cancel",
            Action::Retry => "retry",
        }
    }
}

pub fn apply(job: &Job, action: Action, now: i64) -> Option<Job> {
    use JobState::*;
    let next = match (action, job.state) {
        (Action::Pause, Queued | Running) => Paused,
        (Action::Resume, Paused) => Queued,
        // Cancelling a failed job dismisses it, so "clear" can remove it.
        (Action::Cancel, Queued | Running | Paused | Failed) => Cancelled,
        (Action::Retry, Failed | Cancelled) => {
            // A chapter the source stopped listing gets one more try, final if it fails.
            let unlisted = job.error.as_deref() == Some(UNLISTED);
            return Some(Job {
                error: unlisted.then(|| UNLISTED.to_string()),
                ..requeue(job, now)
            });
        }
        _ => return None,
    };
    Some(Job {
        state: next,
        owner: None,
        not_before: 0,
        updated_at: now,
        ..job.clone()
    })
}

/// A chapter asked for again: `None` while its job is queued or running, a paused job resumes,
/// any other is reset to the back of the queue. `existing` is `None` for a chapter never queued.
/// A `fresh.force` turns on the existing job's force; it never turns it off mid-flight. A running
/// job keeps its owner, which drops the run that keeps old pages and queues it again itself.
pub fn enqueue(existing: Option<&Job>, fresh: Job, now: i64) -> Option<Job> {
    let force = |job: Job| Job {
        force: job.force || fresh.force,
        ..job
    };
    match existing {
        Some(job) if job.state == JobState::Paused => apply(job, Action::Resume, now).map(force),
        Some(job) if job.state == JobState::Queued && fresh.force && !job.force => {
            Some(force(Job {
                updated_at: now,
                ..job.clone()
            }))
        }
        Some(job) if job.state == JobState::Running && fresh.force && !job.force => Some(Job {
            force: true,
            updated_at: now,
            ..job.clone()
        }),
        Some(job) if job.state.is_active() => None,
        Some(job) => Some(Job {
            force: fresh.force,
            ..requeue(job, now)
        }),
        None => Some(Job {
            state: JobState::Queued,
            created_at: now,
            updated_at: now,
            ..fresh
        }),
    }
}

fn requeue(job: &Job, now: i64) -> Job {
    Job {
        state: JobState::Queued,
        attempts: 0,
        error: None,
        pages_done: 0,
        not_before: 0,
        owner: None,
        created_at: now,
        updated_at: now,
        ..job.clone()
    }
}

/// `owner` starts running a queued job: one more attempt.
pub fn claim(job: &Job, owner: &str, now: i64) -> Job {
    Job {
        state: JobState::Running,
        attempts: job.attempts + 1,
        owner: Some(owner.to_string()),
        heartbeat_at: now,
        updated_at: now,
        ..job.clone()
    }
}

/// True while `owner` runs the job; any other state means a pause or cancel won.
pub fn is_run_by(job: &Job, owner: &str) -> bool {
    job.state == JobState::Running && job.owner.as_deref() == Some(owner)
}

/// The owner is alive; records its page count when it changed.
pub fn heartbeat(job: &Job, pages: Option<(usize, usize)>, now: i64) -> Job {
    let mut next = Job {
        heartbeat_at: now,
        ..job.clone()
    };
    if let Some((done, total)) = pages.filter(|p| *p != (job.pages_done, job.pages_total)) {
        next.pages_done = done;
        next.pages_total = total;
        next.updated_at = now;
    }
    next
}

pub fn finish(job: &Job, owner: &str, now: i64) -> Option<Job> {
    is_run_by(job, owner).then(|| Job {
        state: JobState::Done,
        error: None,
        owner: None,
        pages_done: job.pages_total.max(job.pages_done),
        updated_at: now,
        ..job.clone()
    })
}

/// A failed attempt: back to the queue after a backoff while attempts remain and the error can
/// pass by itself, otherwise `Failed`.
pub fn fail(
    job: &Job,
    owner: &str,
    failure: &Failure,
    policy: &RetryPolicy,
    now: i64,
) -> Option<Job> {
    if !is_run_by(job, owner) {
        return None;
    }
    let unlisted = job.error.as_deref() == Some(UNLISTED);
    let again = failure.retryable && !unlisted && job.attempts < policy.max_attempts;
    let message = match unlisted {
        true => format!("not found: {UNLISTED} ({})", failure.message),
        false => failure.message.clone(),
    };
    Some(Job {
        state: if again {
            JobState::Queued
        } else {
            JobState::Failed
        },
        error: Some(message),
        not_before: if again {
            now + backoff(policy, job.attempts)
        } else {
            0
        },
        owner: None,
        updated_at: now,
        ..job.clone()
    })
}

/// A running job whose lease expired `stale_after` ms after its owner's last heartbeat (the
/// process quit or crashed) goes back to the queue with its pages. The interrupted attempt
/// does not count.
pub fn recover(job: &Job, now: i64, stale_after: i64) -> Option<Job> {
    lease_expired(job, now, stale_after).then(|| Job {
        state: JobState::Queued,
        attempts: job.attempts.saturating_sub(1),
        owner: None,
        updated_at: now,
        ..job.clone()
    })
}
