//! Runs this process's queue worker and prints its events until the jobs settle.

use std::sync::Arc;

use mangas_core::download::queue::{
    self, Job, JobState, Queue, QueueConfig, QueueEvent, Scope, Settled,
};
use mangas_core::library::Library;
use mangas_core::Error;
use mangas_providers::QueueRunner;

use crate::output::{out, outln};
use crate::paths;

/// Downloads the jobs in `scope` (other processes may take some of them; they are waited for
/// too, and so is a data-folder move holding the queue) and returns them as they ended.
pub async fn run_until_settled(lib: Arc<Library>, scope: Scope) -> anyhow::Result<Settled> {
    let on_hold = || {
        let _ = outln!("  the data folder is being moved; downloads continue once it's done");
    };
    run_with(lib, scope, &print_event, &on_hold).await
}

/// [`run_until_settled`] printing nothing, for machine-readable output.
pub async fn run_quietly(lib: Arc<Library>, scope: Scope) -> anyhow::Result<Settled> {
    run_with(lib, scope, &|_| {}, &|| {}).await
}

async fn run_with(
    lib: Arc<Library>,
    scope: Scope,
    on_event: &(dyn Fn(QueueEvent) + Send + Sync),
    on_hold: &(dyn Fn() + Send + Sync),
) -> anyhow::Result<Settled> {
    let runner = QueueRunner::new(Box::new(|_| {
        paths::downloads().map_err(|e| Error::Other(e.to_string()))
    }));
    // Chapters left running by a process that died go back to the queue, pages kept.
    let orphans = queue::reclaim_orphans(&lib, queue::now_ms())?;
    if !orphans.is_empty() {
        // stderr: `run_quietly` callers print machine-readable stdout.
        eprintln!(
            "  resuming {} left running by a process that stopped",
            super::plural(orphans.len(), "download", "downloads")
        );
    }
    let worker = Queue::new(lib, Arc::new(runner), QueueConfig::default());
    Ok(queue::run_until_settled(&worker, scope, on_event, on_hold).await?)
}

/// One line per finished job, an updating `page n/total` line while one runs.
fn print_event(event: QueueEvent) {
    // Progress only: a closed stdout must not stop the download.
    let _ = match &event {
        QueueEvent::Progress { job } if job.pages_total > 0 => out!(
            "\r  {}: page {}/{}",
            label(job),
            job.pages_done,
            job.pages_total
        ),
        QueueEvent::Done { job } => outln!(
            "\r  {}: {} pages -> {}",
            label(job),
            job.pages_total,
            job.local_path.as_deref().unwrap_or("?")
        ),
        QueueEvent::Queued { job } => match &job.error {
            Some(error) => outln!("\r  {}: {error}; trying again soon", label(job)),
            None => Ok(()),
        },
        QueueEvent::Failed { job } => {
            eprintln!(
                "\r  {}: {}",
                label(job),
                job.error.as_deref().unwrap_or("failed")
            );
            Ok(())
        }
        QueueEvent::Paused { job } => outln!("\r  {}: paused", label(job)),
        QueueEvent::Cancelled { job } => outln!("\r  {}: cancelled", label(job)),
        QueueEvent::Started { .. }
        | QueueEvent::Progress { .. }
        | QueueEvent::Removed { .. }
        | QueueEvent::Gate { .. } => Ok(()),
    };
}

/// "Berserk ch 12" (the chapter id when the source gives no number).
pub fn label(job: &Job) -> String {
    let number = if job.number.is_empty() {
        job.chapter_id.0.as_str()
    } else {
        job.number.as_str()
    };
    match job.manga_title.as_str() {
        "" => format!("ch {number}"),
        title => format!("{title} ch {number}"),
    }
}

/// How many of `jobs` ended in `state`.
pub fn count(jobs: &[Job], state: JobState) -> usize {
    jobs.iter().filter(|j| j.state == state).count()
}
