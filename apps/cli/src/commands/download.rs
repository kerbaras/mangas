use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::domain::{Chapter, MangaId};
use mangas_core::download::{self, Event};
use mangas_core::library;

use super::source_by_name;
use crate::output::{out, outln};
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
    #[arg(long, default_value = "en")]
    pub lang: String,
    /// Also pack each chapter into a .cbz next to its directory
    #[arg(long)]
    pub cbz: bool,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    if library::is_local(&args.source) {
        bail!("imported manga have nothing to download: their chapters are already on disk");
    }
    let source = source_by_name(&args.source)?;
    let id = MangaId(args.manga_id.clone());
    let lib = library::open(&paths::library_db()?)?;
    if library::get_manga(&lib, &args.source, &id)?.is_none() {
        bail!(
            "{} is not in your library; run `mangas add {}` first",
            id,
            id
        );
    }
    let all = source.chapters(&id, &args.lang).await?;
    library::upsert_chapters(&lib, source.name(), &id, &args.lang, &all)?;
    let selected = match &args.chapters {
        Some(spec) => download::select_chapters(&all, spec)?,
        None => all,
    };
    if selected.is_empty() {
        bail!("no chapters match the selection");
    }
    let selected = without_unavailable(selected, args.chapters.is_some())?;
    outln!("Downloading {} chapter(s) of {}", selected.len(), id)?;
    let root = paths::downloads()?;
    let report =
        download::download_chapters(source.as_ref(), &lib, &selected, &root, &print_event).await?;
    if args.cbz {
        for dir in &report.downloaded {
            let out = download::cbz_path(dir);
            let pages = download::write_cbz(dir, &out)?;
            outln!("packed {pages} pages -> {}", out.display())?;
        }
    }
    if !report.failed.is_empty() {
        bail!("{} chapter(s) failed", report.failed.len());
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

fn print_event(event: Event) {
    match event {
        Event::PageDone { page, total, .. } => {
            // Progress only: a closed stdout must not stop the download.
            let _ = out!("\r  page {page}/{total}");
        }
        Event::ChapterDone { path, pages, .. } => {
            let _ = outln!("\r  {pages} pages -> {path}");
        }
        Event::Error { chapter, message } => eprintln!("\r  {chapter}: {message}"),
    }
}
