use clap::Args as ClapArgs;
use mangas_core::domain::ChapterId;
use mangas_core::download;

use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Chapter ids to download
    #[arg(required = true)]
    pub chapters: Vec<String>,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let root = paths::downloads()?;
    for id in args.chapters {
        let dest = root.join(&id);
        let pages = download::download_chapter(&ChapterId(id.clone()), &dest).await?;
        println!("{id}: {pages} pages -> {}", dest.display());
    }
    Ok(())
}
