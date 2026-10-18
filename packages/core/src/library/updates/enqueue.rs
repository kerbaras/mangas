//! Auto-download: new chapters go to the persistent download queue, never straight to disk.

use std::collections::BTreeSet;

use super::plan::number_key;
use super::{TitleUpdate, UpdateReport};
use crate::domain::{Chapter, Result};
use crate::download::queue::{self, JobId};
use crate::library::Library;

/// The chapters to queue for one title: one per number, in the most preferred language
/// (the check lists languages in preference order) that can actually be downloaded.
pub fn to_download(update: &TitleUpdate) -> Vec<Chapter> {
    let downloadable: BTreeSet<String> = update
        .chapters
        .iter()
        .filter(|c| c.is_available())
        .map(number_key)
        .collect();
    let mut seen = BTreeSet::new();
    update
        .chapters
        .iter()
        .filter(|c| c.is_available() || !downloadable.contains(&number_key(c)))
        .filter(|c| seen.insert(number_key(c)))
        .cloned()
        .collect()
}

/// Queues the new chapters of the titles with auto-download on, or of every title with
/// `everything`. Returns the jobs; the caller nudges or runs the worker.
pub fn enqueue_new(lib: &Library, report: &UpdateReport, everything: bool) -> Result<Vec<JobId>> {
    let mut jobs = Vec::new();
    for update in report
        .updates
        .iter()
        .filter(|u| everything || u.auto_download)
    {
        let queued = queue::enqueue(lib, &update.source, to_download(update), false)?;
        jobs.extend(queued.jobs);
    }
    Ok(jobs)
}
