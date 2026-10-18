use std::path::Path;
use std::sync::Arc;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::domain::{Chapter, MangaId};
use mangas_core::download;
use mangas_core::download::queue::{self, Job, JobState, Scope};
use mangas_core::library;

use super::watch::{count, run_until_settled};
use super::{source_by_name, stored_settings};
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Manga id on the source (must be followed, see `mangas add`)
    pub manga_id: String,
    /// Chapter numbers to download, e.g. "1-5,7" (default: all)
    #[arg(long)]
    pub chapters: Option<String>,
    #[arg(long, default_value = "mangadex")]
    pub source: String,
    /// Chapter languages, most preferred first: each chapter comes in the first one that has
    /// it, e.g. "es,en" [default: your preferred languages]
    #[arg(long)]
    pub lang: Option<String>,
    /// Also pack each chapter into a .cbz next to its directory
    #[arg(long)]
    pub cbz: bool,
    /// Also queue chapters already on disk; their missing pages get downloaded
    #[arg(long)]
    pub force: bool,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    if library::is_local(&args.source) {
        bail!("imported manga have nothing to download: their chapters are already on disk");
    }
    let source = source_by_name(&args.source)?;
    let id = MangaId(args.manga_id.clone());
    let lib = Arc::new(library::open(&paths::library_db()?)?);
    if library::get_manga(&lib, &args.source, &id)?.is_none() {
        bail!(
            "{} is not in your library; run `mangas add {}` first",
            id,
            id
        );
    }
    let requested = library::requested_languages(args.lang.as_deref(), &stored_settings());
    let languages = mangas_providers::languages_for(&args.source, &requested);
    let synced = library::sync_chapters(&lib, source.as_ref(), &id, &languages).await?;
    let all = library::prefer_languages(&synced, &languages);
    let selected = match &args.chapters {
        Some(spec) => download::select_chapters(&all, spec)?,
        None => all,
    };
    if selected.is_empty() {
        bail!("no chapters match the selection");
    }
    let selected = without_unavailable(selected, args.chapters.is_some())?;
    let queued = queue::enqueue(&lib, &args.source, selected, args.force)?;
    if !queued.on_disk.is_empty() {
        outln!(
            "skipped {} chapter(s) already on disk (--force downloads them again)",
            queued.on_disk.len()
        )?;
    }
    if queued.jobs.is_empty() {
        return Ok(());
    }
    outln!("Downloading {} chapter(s) of {}", queued.jobs.len(), id)?;
    let jobs = run_until_settled(lib, Scope::Jobs(queued.jobs.into_iter().collect())).await?;
    if args.cbz {
        for job in jobs.iter().filter(|j| j.state == JobState::Done) {
            if let Some(dir) = &job.local_path {
                let out = download::cbz_path(Path::new(dir));
                let pages = download::write_cbz(Path::new(dir), &out)?;
                outln!("packed {pages} pages -> {}", out.display())?;
            }
        }
    }
    if let Some(note) = waiting_note(&jobs) {
        outln!("{note}")?;
    }
    let failed = count(&jobs, JobState::Failed);
    if failed > 0 {
        bail!("{failed} chapter(s) failed; `mangas queue retry --all` tries them again");
    }
    Ok(())
}

/// Drops locked and external chapters with a summary line; an explicit selection also names
/// each one. Nothing left to download is an error that gives the reason.
fn without_unavailable(selected: Vec<Chapter>, explicit: bool) -> anyhow::Result<Vec<Chapter>> {
    let (available, skipped) = download::split_available(selected);
    if let [only] = skipped.as_slice() {
        if available.is_empty() {
            download::ensure_available(only)?;
        }
    }
    let reasons = skipped
        .iter()
        .filter_map(|c| download::ensure_available(c).err());
    if explicit {
        reasons.for_each(|e| eprintln!("  {e}"));
    }
    if let Some(summary) = download::skipped_summary(&skipped) {
        if available.is_empty() {
            bail!("nothing to download: {summary}");
        }
        outln!("{summary}")?;
    }
    Ok(available)
}

/// Chapters left undone without failing: paused ones, or queued ones held by the queue-wide
/// pause (the worker stops once nothing more can start).
fn waiting_note(jobs: &[Job]) -> Option<String> {
    let (paused, held) = (count(jobs, JobState::Paused), count(jobs, JobState::Queued));
    let what = match (paused, held) {
        (0, 0) => return None,
        (p, 0) => format!("{p} chapter(s) paused"),
        (0, q) => format!("{q} chapter(s) waiting: the download queue is paused"),
        (p, q) => format!("{p} chapter(s) paused and {q} waiting: the download queue is paused"),
    };
    Some(format!(
        "{what}; `mangas queue resume --all` then `mangas queue run` continue them"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(state: JobState) -> Job {
        Job {
            state,
            ..Default::default()
        }
    }

    #[test]
    fn chapters_held_by_the_queue_pause_are_reported_not_passed_off_as_done() {
        assert_eq!(waiting_note(&[job(JobState::Done)]), None);
        let held = waiting_note(&[job(JobState::Done), job(JobState::Queued)]).unwrap();
        assert!(
            held.starts_with("1 chapter(s) waiting: the download queue is paused"),
            "{held}"
        );
        let paused = waiting_note(&[job(JobState::Paused)]).unwrap();
        assert!(paused.starts_with("1 chapter(s) paused;"), "{paused}");
    }
}
