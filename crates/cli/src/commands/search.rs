use clap::Args as ClapArgs;

use super::source_by_name;

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
    let source = source_by_name(&args.source)?;
    let results = source.search(&args.query, args.limit).await?;
    if results.is_empty() {
        println!("No results for \"{}\" on {}", args.query, args.source);
        return Ok(());
    }
    for m in results {
        println!(
            "{:<36}  {:<10}  {}",
            m.id,
            format!("{:?}", m.status).to_lowercase(),
            m.title
        );
    }
    Ok(())
}
