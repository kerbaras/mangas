use std::path::PathBuf;

use clap::Args as ClapArgs;
use mangas_core::library::{self, ImportRequest};

use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// A series folder (chapter folders or .cbz files inside), or chapter folders / .cbz files
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,
    /// Series title [default: the folder name]
    #[arg(long)]
    pub title: Option<String>,
    /// Language the chapters are stored under
    #[arg(long, default_value = "en")]
    pub lang: String,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let req = ImportRequest {
        paths: args.paths,
        title: args.title,
        language: args.lang,
    };
    let report = library::import_paths(&lib, &paths::downloads()?, &req)?;
    for c in &report.imported {
        let number = if c.number.is_empty() { "-" } else { &c.number };
        println!("  {number:<8} {:>4} pages -> {}", c.pages, c.path.display());
    }
    for s in &report.skipped {
        eprintln!("  skipped {}: {}", s.name, s.reason);
    }
    println!("{}", report.summary());
    println!(
        "Library id: {} (source {})",
        report.manga.id, report.manga.source
    );
    Ok(())
}
