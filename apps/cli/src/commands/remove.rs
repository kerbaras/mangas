use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::domain::MangaId;
use mangas_core::library;

use super::followed::followed_source;
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Manga id on the source (see `mangas list`)
    pub id: String,
    /// Source of the title [default: the one your library follows it on]
    #[arg(long)]
    pub source: Option<String>,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let id = MangaId(args.id.clone());
    let source = followed_source(&lib, &id, args.source.as_deref())?;
    if !library::remove_manga(&lib, &source, &id)? {
        bail!("{} is not in your library", args.id);
    }
    outln!("Removed {} (downloaded files were kept)", args.id)?;
    Ok(())
}
