//! One claimed job running as a task: its page count is shared with the worker loop, which is
//! woken on every page landed so the count is saved (and reported) right away.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::{mpsc, Notify};
use tokio::task::JoinHandle;

use super::job::{Job, JobId};
use super::retry::{classify, Failure};
use super::runner::JobRunner;
use crate::download::Event;
use crate::library::Library;

/// `(pages done, total)` per running job, as last reported by its task.
pub(super) type Progress = Arc<Mutex<HashMap<JobId, (usize, usize)>>>;
/// A task's result, tagged with the run it belongs to: a job paused and resumed between two
/// ticks is claimed again while its old task is still winding down.
pub(super) type Outcome = (JobId, u64, Option<Failure>);

pub(super) struct Task {
    pub lib: Arc<Library>,
    pub runner: Arc<dyn JobRunner>,
    pub progress: Progress,
    /// The worker loop's wake-up: a landed page makes it step now instead of at the next tick.
    pub wake: Arc<Notify>,
}

impl Task {
    pub fn spawn(self, job: Job, run: u64, tx: mpsc::UnboundedSender<Outcome>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let done = AtomicUsize::new(0);
            let on_event = |event: Event| {
                if let Event::PageDone { total, .. } = event {
                    let count = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if let Ok(mut p) = self.progress.lock() {
                        p.insert(job.id, (count.min(total), total));
                    }
                    self.wake.notify_one();
                }
            };
            let outcome = self.runner.run(&self.lib, &job, &on_event).await.err();
            if let Some(e) = &outcome {
                let (id, chapter) = (job.id, &job.chapter_id);
                log::warn!("download {id} (chapter {chapter}) failed: {e}");
            }
            let _ = tx.send((job.id, run, outcome.as_ref().map(classify)));
        })
    }
}
