//! The queue's async edge: one loop per process that steps the queue ([`plan_tick`]), runs the
//! claimed jobs as tasks, stops the ones paused or cancelled from anywhere, and reports changes.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, Notify};
use tokio::task::JoinHandle;

use super::events::{diff, gate_event, QueueEvent};
use super::job::{GateStatus, Job, JobId};
use super::lease::{new_owner, Clock};
use super::ops::now_ms;
use super::retry::Failure;
use super::runner::JobRunner;
use super::schedule::{is_settled, Scope};
use super::task::{Outcome, Progress, Task};
use super::tick::{plan_tick, QueueConfig, TickInput, TickPlan};
use super::transitions::is_run_by;
use crate::domain::Result;
use crate::library::{self, Library};

type Tasks = HashMap<JobId, (u64, bool, JoinHandle<()>)>;

/// Sink for queue events; called from the worker loop only.
pub type OnQueueEvent<'a> = &'a (dyn Fn(QueueEvent) + Send + Sync);

pub struct Queue {
    lib: Arc<Library>,
    runner: Arc<dyn JobRunner>,
    config: QueueConfig,
    owner: String,
    wake: Arc<Notify>,
    clock: Clock,
}

impl Queue {
    pub fn new(lib: Arc<Library>, runner: Arc<dyn JobRunner>, config: QueueConfig) -> Self {
        Queue {
            lib,
            runner,
            config,
            owner: new_owner(),
            wake: Arc::default(),
            clock: Arc::new(now_ms),
        }
    }

    /// Uses `clock` for every time decision (claims, heartbeats, lease expiry, backoff).
    pub fn with_clock(self, clock: Clock) -> Self {
        Queue { clock, ..self }
    }

    pub fn library(&self) -> &Library {
        &self.lib
    }

    /// Makes the worker step now instead of at its next tick (after a command changed jobs).
    pub fn nudge(&self) {
        self.wake.notify_one();
    }

    /// Runs the jobs in `scope` until `settle` is set and none of them is queued or running
    /// any more (the CLI); without `settle` it runs for as long as the process does.
    pub async fn run(&self, scope: Scope, settle: bool, on_event: OnQueueEvent<'_>) -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<Outcome>();
        let progress: Progress = Arc::default();
        let mut tasks: Tasks = HashMap::new();
        let mut arrived: Vec<Outcome> = Vec::new();
        let mut finished: Vec<(JobId, Option<Failure>)> = Vec::new();
        let mut runs: u64 = 0;
        let mut prev = library::list_jobs(&self.lib)?;
        let mut gate: Option<GateStatus> = None;
        loop {
            while let Ok(outcome) = rx.try_recv() {
                arrived.push(outcome);
            }
            for (id, run, outcome) in arrived.drain(..) {
                if tasks.get(&id).is_some_and(|(current, ..)| *current == run) {
                    tasks.remove(&id);
                    finished.push((id, outcome));
                }
            }
            match self.step(&scope, &tasks, &progress, &finished) {
                Ok((plan, status)) => {
                    if let Ok(mut p) = progress.lock() {
                        for (id, _) in &finished {
                            p.remove(id);
                        }
                    }
                    finished.clear();
                    for job in plan.claimed {
                        // Paused and resumed since the last step: the old run must be gone
                        // before the new one writes the same pages.
                        if let Some((.., old)) = tasks.remove(&job.id) {
                            old.abort();
                            let _ = old.await;
                        }
                        runs += 1;
                        let task = Task {
                            lib: self.lib.clone(),
                            runner: self.runner.clone(),
                            progress: progress.clone(),
                            wake: self.wake.clone(),
                        };
                        let handle = task.spawn(job.clone(), runs, tx.clone());
                        tasks.insert(job.id, (runs, job.force, handle));
                    }
                    self.stop_lost(&plan.jobs, &mut tasks, &progress);
                    diff(&prev, &plan.jobs).into_iter().for_each(on_event);
                    gate_event(gate, status).into_iter().for_each(on_event);
                    prev = plan.jobs;
                    gate = Some(status);
                }
                Err(e) => log::warn!("download queue: {e}"),
            }
            // With the queue-wide gate closed nothing more will start: a settling worker stops.
            let held = gate.is_some_and(|g| g.paused || g.held);
            if settle && tasks.is_empty() && (held || is_settled(&prev, &scope)) {
                return Ok(());
            }
            tokio::select! {
                _ = tokio::time::sleep(self.config.tick) => {}
                _ = self.wake.notified() => {}
                Some(outcome) = rx.recv() => arrived.push(outcome),
            }
        }
    }

    fn step(
        &self,
        scope: &Scope,
        tasks: &Tasks,
        progress: &Progress,
        finished: &[(JobId, Option<Failure>)],
    ) -> Result<(TickPlan, GateStatus)> {
        let pages = progress.lock().map(|p| p.clone()).unwrap_or_default();
        let in_flight = tasks
            .keys()
            .chain(finished.iter().map(|(id, _)| id))
            .map(|id| (*id, pages.get(id).copied()))
            .collect();
        let input = TickInput {
            owner: &self.owner,
            now: (self.clock)(),
            in_flight: &in_flight,
            finished,
            scope,
            config: &self.config,
            paused: false,
        };
        library::update_jobs_gated(&self.lib, |jobs, gate| {
            let paused = gate.closed(input.now);
            let plan = plan_tick(jobs, &TickInput { paused, ..input });
            (plan.changed.clone(), (plan, gate.status(input.now)))
        })
    }

    /// Aborts tasks whose job is no longer running under this worker (paused or cancelled), or
    /// was forced since it started: the next step queues it again for a full re-download.
    fn stop_lost(&self, jobs: &[Job], tasks: &mut Tasks, progress: &Progress) {
        tasks.retain(|id, (_, forced, handle)| {
            let ours = jobs
                .iter()
                .any(|j| j.id == *id && is_run_by(j, &self.owner) && (*forced || !j.force));
            if !ours {
                handle.abort();
                let _ = progress.lock().map(|mut p| p.remove(id));
            }
            ours
        });
    }
}
