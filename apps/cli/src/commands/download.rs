use std::path::Path;
use std::sync::Arc;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::domain::{Chapter, MangaId};
use mangas_core::download;
use mangas_core::download::queue::{self, JobState, Scope};
use mangas_core::library;

use super::download_releases::{pick, pick_group, report_left_out};
use super::settled::{failed_note, paused_report};
use super::watch::run_until_settled;
use super::{hint, source_by_name, stored_settings, unknown_group};
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
    /// Also queue chapters already on disk and download every page again, overwriting the files
    #[arg(long)]
    pub force: bool,
    /// Only chapters of this scanlation group, instead of the one saved for the title
    /// (`mangas group`)
    #[arg(long, conflicts_with = "all_groups")]
    pub group: Option<String>,
    /// Every group's release of each chapter, ignoring the group saved for the title (without
    /// a group, one release per chapter is taken, preferring the provider's preferred groups)
    #[arg(long)]
    pub all_groups: bool,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    queue::ensure_downloadable(&args.source)?;
    let source = source_by_name(&args.source)?;
    let id = MangaId(args.manga_id.clone());
    let lib = Arc::new(library::open(&paths::library_db()?)?);
    if library::get_manga(&lib, &args.source, &id)?.is_none() {
        let add = hint(&format!("add {id}"), &args.source);
        bail!("{id} is not in your library; run {add} first");
    }
    let settings = stored_settings();
    let requested = library::requested_languages(args.lang.as_deref(), &settings);
    let languages = mangas_providers::languages_for(&args.source, &requested);
    let synced = library::sync_chapters(&lib, source.as_ref(), &id, &languages).await?;
    let saved = library::chapter_group(&lib, &args.source, &id)?;
    let group = pick_group(args.group.as_deref(), args.all_groups, saved);
    let picked = pick(
        &lib,
        &args.source,
        &synced,
        group.as_deref(),
        args.all_groups,
        &settings,
    )?;
    if let (Some(group), true) = (&group, picked.is_empty()) {
        return Err(unknown_group(group, &id, &args.source));
    }
    if let Some(group) = &group {
        outln!("Group: {group}")?;
    }
    let all = library::prefer_languages(&picked, &languages);
    let selected = match &args.chapters {
        Some(spec) => download::select_chapters(&all, spec)?,
        None => all,
    };
    if selected.is_empty() {
        bail!("no chapters match the selection");
    }
    if group.is_none() && !args.all_groups {
        report_left_out(&synced, &selected, &id, &args.source, &settings)?;
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
    let settled = run_until_settled(lib, Scope::Jobs(queued.jobs.into_iter().collect())).await?;
    let jobs = &settled.jobs;
    if args.cbz {
        for job in jobs.iter().filter(|j| j.state == JobState::Done) {
            if let Some(dir) = &job.local_path {
                let out = download::cbz_path(Path::new(dir));
                let pages = download::write_cbz(Path::new(dir), &out)?;
                outln!("packed {pages} pages -> {}", out.display())?;
            }
        }
    }
    let failed_note = failed_note(jobs, "chapter");
    if let Some((lines, exit)) = paused_report(&settled) {
        for line in lines {
            outln!("{line}")?;
        }
        if let Some(note) = &failed_note {
            eprintln!("{note}");
        }
        return Err(exit.into());
    }
    if let Some(note) = failed_note {
        bail!("{note}");
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
