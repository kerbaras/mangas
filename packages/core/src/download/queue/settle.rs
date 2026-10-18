//! Running a set of jobs to the end for the CLI, through a storage move's hold.

use std::time::Duration;

use super::job::{Job, JobState};
use super::ops::{list, now_ms};
use super::schedule::Scope;
use super::worker::{OnQueueEvent, Queue};
use crate::domain::Result;
use crate::library;

const HELD_POLL: Duration = Duration::from_millis(500);

/// How a settling run ended: the jobs in scope, and whether the user's queue-wide pause was
/// set (`queue pause --all` also pauses each job, so the ones left are queued or paused).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    pub jobs: Vec<Job>,
    pub paused: bool,
}

/// Runs `scope` until none of it is queued or running, in this process or another. A storage
/// move's hold is waited out (`on_hold` is called once per hold) and the jobs carry on; the
/// user's pause ends the run, once nothing is running, with jobs still queued.
pub async fn run_until_settled(
    worker: &Queue,
    scope: Scope,
    on_event: OnQueueEvent<'_>,
    on_hold: &(dyn Fn() + Send + Sync),
) -> Result<Settled> {
    let mut waiting_out_hold = false;
    loop {
        worker.run(scope.clone(), true, on_event).await?;
        let lib = worker.library();
        let jobs: Vec<Job> = list(lib)?
            .into_iter()
            .filter(|j| scope.contains(j.id))
            .collect();
        let gate = library::queue_gate(lib)?;
        let any = |state: JobState| jobs.iter().any(|j| j.state == state);
        // A chapter running in another process is not done yet, whatever the gate says.
        if !any(JobState::Running) && (gate.paused || !any(JobState::Queued)) {
            let paused = gate.paused;
            return Ok(Settled { jobs, paused });
        }
        let now = now_ms();
        if gate.held(now) && !waiting_out_hold {
            on_hold();
        }
        waiting_out_hold = gate.held(now);
        if gate.closed(now) {
            tokio::time::sleep(HELD_POLL).await;
        }
    }
}
