//! The queue's entry points for the apps: enqueue, pause/resume/cancel/retry, clear, list.

use super::job::{Job, JobId, JobState};
use super::transitions::{apply, Action};
use crate::domain::{Chapter, Error, Result};
use crate::download::split_available;
use crate::library::{self, Library};

/// What an enqueue did with each chapter it was given.
#[derive(Debug, Default)]
pub struct Enqueued {
    /// Jobs of the queued chapters, including ones that were already on their way.
    pub jobs: Vec<JobId>,
    /// Already downloaded and left alone (pass `force` to queue them anyway).
    pub on_disk: Vec<Chapter>,
    /// Locked or hosted elsewhere: nothing to download.
    pub unavailable: Vec<Chapter>,
}

/// Splits `chapters` into `(to queue, already on disk, unavailable)`, each in order. With
/// `force` a downloaded chapter is queued again, which fills in any page missing on disk.
pub fn split_for_queue(
    chapters: Vec<Chapter>,
    force: bool,
) -> (Vec<Chapter>, Vec<Chapter>, Vec<Chapter>) {
    let (available, unavailable) = split_available(chapters);
    let (on_disk, to_queue) = available
        .into_iter()
        .partition(|c| c.is_downloaded() && !force);
    (to_queue, on_disk, unavailable)
}

/// Queues the library chapters `chapters` of `source`. Whether one is on disk is read from the
/// library, so a list fresh from the source still skips what was downloaded before.
pub fn enqueue(
    lib: &Library,
    source: &str,
    chapters: Vec<Chapter>,
    force: bool,
) -> Result<Enqueued> {
    let chapters = with_stored_paths(lib, source, chapters)?;
    let (to_queue, on_disk, unavailable) = split_for_queue(chapters, force);
    let ids: Vec<_> = to_queue.iter().map(|c| c.id.clone()).collect();
    let jobs = library::enqueue_jobs(lib, source, &ids, now_ms())?;
    Ok(Enqueued {
        jobs,
        on_disk,
        unavailable,
    })
}

fn with_stored_paths(lib: &Library, source: &str, chapters: Vec<Chapter>) -> Result<Vec<Chapter>> {
    let mut stored: Vec<Chapter> = Vec::new();
    for manga in chapters.iter().map(|c| &c.manga_id) {
        if !stored.iter().any(|c| &c.manga_id == manga) {
            stored.extend(library::list_chapters(lib, source, manga, None)?);
        }
    }
    Ok(chapters
        .into_iter()
        .map(|c| match stored.iter().find(|s| s.id == c.id) {
            Some(s) if c.local_path.is_none() => Chapter {
                local_path: s.local_path.clone(),
                ..c
            },
            _ => c,
        })
        .collect())
}

/// Which jobs an action is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Job(JobId),
    /// Every job the action applies to (pause all, retry all failed, ...).
    All,
}

/// Applies `action` and returns the jobs it changed. One job that is unknown, or in a state
/// the action does not apply to, is an error that says so; `All` just skips those.
pub fn control(lib: &Library, target: Target, action: Action) -> Result<Vec<Job>> {
    let now = now_ms();
    // Pause all also stops new claims, in every process, until resume all.
    match (target, action) {
        (Target::All, Action::Pause) => library::set_queue_paused(lib, true)?,
        (Target::All, Action::Resume) => library::set_queue_paused(lib, false)?,
        _ => {}
    }
    library::update_jobs(lib, |jobs| {
        let changed: Vec<Job> = jobs
            .iter()
            .filter(|j| target == Target::All || target == Target::Job(j.id))
            .filter_map(|j| apply(j, action, now))
            .collect();
        let outcome = match target {
            Target::Job(id) if changed.is_empty() => Err(refusal(&jobs, id, action)),
            _ => Ok(changed.clone()),
        };
        (changed, outcome)
    })?
}

fn refusal(jobs: &[Job], id: JobId, action: Action) -> Error {
    match jobs.iter().find(|j| j.id == id) {
        Some(job) => Error::Other(format!(
            "download {id} is {}; nothing to {}",
            job.state.as_str(),
            action.verb()
        )),
        None => Error::NotFound(format!("download {id}")),
    }
}

/// Removes finished (`done`) and cancelled jobs; returns how many.
pub fn clear_finished(lib: &Library) -> Result<usize> {
    library::delete_jobs(lib, &[JobState::Done, JobState::Cancelled])
}

/// Every job in queue order.
pub fn list(lib: &Library) -> Result<Vec<Job>> {
    library::list_jobs(lib)
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
