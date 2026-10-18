use clap::Args as ClapArgs;
use mangas_core::domain::MangaId;
use mangas_core::library;

use super::{source_by_name, stored_settings};
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Manga id on the source (see `mangas search`)
    pub id: String,
    #[arg(long, default_value = "mangadex")]
    pub source: String,
    /// Chapter languages to track, e.g. "es,en" [default: your preferred languages]
    #[arg(long)]
    pub lang: Option<String>,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let source = source_by_name(&args.source)?;
    let id = MangaId(args.id);
    let manga = source.manga(&id).await?;
    let requested = library::requested_languages(args.lang.as_deref(), &stored_settings());
    let languages = mangas_providers::languages_for(&args.source, &requested);
    // Fetched before anything is saved: a failed add must not leave the manga followed.
    let lists = library::fetch_chapters(source.as_ref(), &id, &languages).await?;
    let lib = library::open(&paths::library_db()?)?;
    library::upsert_manga(&lib, &manga)?;
    let chapters = library::store_chapters(&lib, source.name(), &id, lists)?;
    let unavailable = chapters.iter().filter(|c| !c.is_available()).count();
    let note = match unavailable {
        0 => String::new(),
        n => format!(", {n} locked or on another site"),
    };
    outln!(
        "Following {} ({} {} chapters{note})",
        manga.title,
        chapters.len(),
        languages.join("/")
    )?;
    Ok(())
}
