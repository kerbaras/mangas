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
    let languages = library::requested_languages(args.lang.as_deref(), &stored_settings());
    let lib = library::open(&paths::library_db()?)?;
    library::upsert_manga(&lib, &manga)?;
    let chapters = library::sync_chapters(&lib, source.as_ref(), &id, &languages).await?;
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
