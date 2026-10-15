use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::domain::MangaId;
use mangas_core::{library, sources};

use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Manga id on the source (see `mangas search`)
    pub id: String,
    #[arg(long, default_value = "mangadex")]
    pub source: String,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let Some(source) = sources::by_name(&args.source) else {
        bail!("unknown source '{}'", args.source);
    };
    let manga = source.manga(&MangaId(args.id)).await?;
    let lib = library::open(&paths::library_db()?)?;
    library::upsert_manga(&lib, &manga)?;
    println!("Following {}", manga.title);
    Ok(())
}
