use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::sources;

#[derive(ClapArgs)]
pub struct Args {
    /// Title to search for
    pub query: String,
    /// Source to search (default: mangadex)
    #[arg(long, default_value = "mangadex")]
    pub source: String,
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let Some(source) = sources::by_name(&args.source) else {
        bail!("unknown source '{}'", args.source);
    };
    for m in source.search(&args.query, args.limit).await? {
        println!("{:<36}  {}", m.id, m.title);
    }
    Ok(())
}
