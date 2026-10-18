//! Auto-download: new chapters go to the persistent download queue, never straight to disk.

use std::collections::BTreeSet;

use super::plan::number_key;
use super::{TitleUpdate, UpdateReport};
use crate::domain::{collapse_groups, in_group, Chapter, Result};
use crate::download::queue::{self, JobId};
use crate::library::{get_settings, Library};

/// The chapters to queue for one title: one per number, in the most preferred language
/// (the check lists languages in preference order) that can actually be downloaded, from the
/// title's group when it is narrowed to one, else the release `preferred` groups favour.
pub fn to_download(update: &TitleUpdate, preferred: &[String]) -> Vec<Chapter> {
    let chapters = match update.chapter_group.as_deref() {
        Some(group) => in_group(update.chapters.clone(), Some(group)),
        None => collapse_groups(update.chapters.clone(), preferred),
    };
    let downloadable: BTreeSet<String> = chapters
        .iter()
        .filter(|c| c.is_available())
        .map(number_key)
        .collect();
    let mut seen = BTreeSet::new();
    chapters
        .into_iter()
        .filter(|c| c.is_available() || !downloadable.contains(&number_key(c)))
        .filter(|c| seen.insert(number_key(c)))
        .collect()
}

/// Queues the new chapters of the titles with auto-download on, or of every title with
/// `everything`. Returns the jobs; the caller nudges or runs the worker.
pub fn enqueue_new(lib: &Library, report: &UpdateReport, everything: bool) -> Result<Vec<JobId>> {
    let settings = get_settings(lib)?;
    let mut jobs = Vec::new();
    for update in report
        .updates
        .iter()
        .filter(|u| everything || u.auto_download)
    {
        let queued = queue::enqueue(
            lib,
            &update.source,
            to_download(update, &settings.preferred_groups(&update.source)),
            false,
        )?;
        jobs.extend(queued.jobs);
    }
    Ok(jobs)
}
