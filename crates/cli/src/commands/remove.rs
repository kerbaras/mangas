use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::domain::MangaId;
use mangas_core::library;

use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Manga id on the source (see `mangas list`)
    pub id: String,
    #[arg(long, default_value = "mangadex")]
    pub source: String,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    if !library::remove_manga(&lib, &args.source, &MangaId(args.id.clone()))? {
        bail!("{} is not in your library", args.id);
    }
    println!("Removed {} (downloaded files were kept)", args.id);
    Ok(())
}
