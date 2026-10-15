use clap::Args as ClapArgs;
use mangas_core::domain::MangaId;
use mangas_core::library;

use super::source_by_name;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Manga id on the source (see `mangas search`)
    pub id: String,
    #[arg(long, default_value = "mangadex")]
    pub source: String,
    /// Chapter language to track
    #[arg(long, default_value = "en")]
    pub lang: String,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let source = source_by_name(&args.source)?;
    let id = MangaId(args.id);
    let manga = source.manga(&id).await?;
    let chapters = source.chapters(&id, &args.lang).await?;
    let lib = library::open(&paths::library_db()?)?;
    library::upsert_manga(&lib, &manga)?;
    library::upsert_chapters(&lib, source.name(), &chapters)?;
    println!(
        "Following {} ({} {} chapters available)",
        manga.title,
        chapters.len(),
        args.lang
    );
    Ok(())
}
