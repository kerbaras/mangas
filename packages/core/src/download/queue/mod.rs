//! Persistent download queue. Jobs live in the library's `download_jobs` table so they survive
//! restarts and are shared by the desktop app and the CLI. Everything but [`worker`] is pure:
//! [`transitions`] for one job, [`schedule`] for which jobs start, [`tick`] for one worker
//! step, [`events`] for what changed.

mod events;
mod hold;
mod job;
mod lease;
mod ops;
mod releases;
mod retry;
mod runner;
mod schedule;
mod settle;
mod task;
mod tick;
mod transitions;
mod worker;

/// Why a queued download failed when a refresh found its chapter gone from the source.
pub const UNLISTED: &str = "chapter no longer listed by the source";

pub use events::{diff, gate_event, QueueEvent};
pub use hold::{hold_queue, hold_queue_at, MOVE_SETTLE};
pub use job::{GateStatus, Job, JobId, JobState, QueueGate};
pub use lease::{holds_lease, lease_expired, mark_stale, Clock, LEASE_MS, STALL_MS};
pub use ops::{
    clear_finished, control, dismiss, enqueue, find, gate_status, list, now_ms, reclaim_orphans,
    split_for_queue, Enqueued, Target,
};
pub use releases::pick_releases;
pub use retry::{backoff, classify, Failure, RetryPolicy};
pub use runner::{run_job, JobRunner};
pub use schedule::{in_queue_order, is_settled, pick, Limits, Scope};
pub use settle::{run_until_settled, Settled};
pub use tick::{plan_tick, QueueConfig, TickInput, TickPlan};
pub use transitions::{
    apply, claim, enqueue as enqueue_job, fail, finish, heartbeat, is_run_by, recover, Action,
};
pub use worker::{OnQueueEvent, Queue};
