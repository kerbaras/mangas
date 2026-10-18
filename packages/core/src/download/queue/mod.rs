//! Persistent download queue. Jobs live in the library's `download_jobs` table so they survive
//! restarts and are shared by the desktop app and the CLI. Everything but [`worker`] is pure:
//! [`transitions`] for one job, [`schedule`] for which jobs start, [`tick`] for one worker
//! step, [`events`] for what changed.

mod events;
mod job;
mod ops;
mod retry;
mod runner;
mod schedule;
mod tick;
mod transitions;
mod worker;

/// Why a queued download failed when a refresh found its chapter gone from the source.
pub const UNLISTED: &str = "chapter no longer listed by the source";

pub use events::{diff, QueueEvent};
pub use job::{Job, JobId, JobState};
pub use ops::{
    clear_finished, control, enqueue, ensure_idle, in_progress, list, now_ms, split_for_queue,
    Enqueued, Target,
};
pub use retry::{backoff, classify, Failure, RetryPolicy};
pub use runner::{run_job, JobRunner};
pub use schedule::{in_queue_order, pick, Limits, Scope};
pub use tick::{plan_tick, QueueConfig, TickInput, TickPlan};
pub use transitions::{
    apply, claim, enqueue as enqueue_job, fail, finish, heartbeat, is_run_by, recover, Action,
};
pub use worker::{is_settled, OnQueueEvent, Queue};
