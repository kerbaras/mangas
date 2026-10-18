//! Queue events for the UIs, derived by diffing two snapshots of the queue, so a change made
//! by another process (the CLI pausing a job the app runs) reaches the app the same way.

use serde::{Deserialize, Serialize};

use super::job::{GateStatus, Job, JobId, JobState};

/// Serialised as `{"kind": "progress", "job": {...}}`; every event but `removed` carries the
/// whole job as it is now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QueueEvent {
    Queued {
        job: Job,
    },
    Started {
        job: Job,
    },
    Progress {
        job: Job,
    },
    Paused {
        job: Job,
    },
    Done {
        job: Job,
    },
    Failed {
        job: Job,
    },
    Cancelled {
        job: Job,
    },
    Removed {
        id: JobId,
    },
    /// The queue-wide gate changed (pause all, resume all, a data-folder move's hold).
    Gate {
        paused: bool,
        held: bool,
    },
}

impl QueueEvent {
    pub fn job(&self) -> Option<&Job> {
        match self {
            QueueEvent::Queued { job }
            | QueueEvent::Started { job }
            | QueueEvent::Progress { job }
            | QueueEvent::Paused { job }
            | QueueEvent::Done { job }
            | QueueEvent::Failed { job }
            | QueueEvent::Cancelled { job } => Some(job),
            QueueEvent::Removed { .. } | QueueEvent::Gate { .. } => None,
        }
    }
}

/// Events that turn `prev` into `next`. Heartbeats alone are not news.
pub fn diff(prev: &[Job], next: &[Job]) -> Vec<QueueEvent> {
    let changed = next.iter().filter_map(|job| {
        let before = prev.iter().find(|p| p.id == job.id);
        match before {
            Some(p) if same(p, job) => None,
            Some(p) if p.state == JobState::Running && job.state == JobState::Running => {
                Some(QueueEvent::Progress { job: job.clone() })
            }
            _ => Some(state_event(job.clone())),
        }
    });
    let removed = prev
        .iter()
        .filter(|p| !next.iter().any(|j| j.id == p.id))
        .map(|p| QueueEvent::Removed { id: p.id });
    changed.chain(removed).collect()
}

/// The gate event to send when the gate reads `next` and last read `prev` (`None`: first read).
pub fn gate_event(prev: Option<GateStatus>, next: GateStatus) -> Option<QueueEvent> {
    (prev != Some(next)).then_some(QueueEvent::Gate {
        paused: next.paused,
        held: next.held,
    })
}

fn same(a: &Job, b: &Job) -> bool {
    Job {
        heartbeat_at: 0,
        ..a.clone()
    } == Job {
        heartbeat_at: 0,
        ..b.clone()
    }
}

fn state_event(job: Job) -> QueueEvent {
    match job.state {
        JobState::Queued => QueueEvent::Queued { job },
        JobState::Running => QueueEvent::Started { job },
        JobState::Paused => QueueEvent::Paused { job },
        JobState::Done => QueueEvent::Done { job },
        JobState::Failed => QueueEvent::Failed { job },
        JobState::Cancelled => QueueEvent::Cancelled { job },
    }
}
