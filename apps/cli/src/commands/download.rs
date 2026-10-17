use std::io::Write;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::domain::MangaId;
use mangas_core::download::{self, Event};
use mangas_core::library;

use super::source_by_name;
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
    println!("Downloading {} chapter(s) of {}", selected.len(), id);
    let root = paths::downloads()?;
    let report =
        download::download_chapters(source.as_ref(), &lib, &selected, &root, &print_event).await?;
    if args.cbz {
        for dir in &report.downloaded {
            let out = download::cbz_path(dir);
            let pages = download::write_cbz(dir, &out)?;
            println!("packed {pages} pages -> {}", out.display());
        }
    }
    if !report.failed.is_empty() {
        bail!("{} chapter(s) failed", report.failed.len());
    }
    Ok(())
}

fn print_event(event: Event) {
    match event {
        Event::PageDone { page, total, .. } => {
            print!("\r  page {page}/{total}");
            let _ = std::io::stdout().flush();
        }
        Event::ChapterDone { path, pages, .. } => println!("\r  {pages} pages -> {path}"),
        Event::Error { chapter, message } => eprintln!("\r  {chapter}: {message}"),
    }
}
