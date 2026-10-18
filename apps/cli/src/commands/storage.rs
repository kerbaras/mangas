use std::path::PathBuf;

use clap::{Args as ClapArgs, Subcommand, ValueEnum};
use mangas_core::library;
use mangas_core::storage::{self, human_size, Cache, Usage};

use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show how much space each folder and each title takes (the default)
    Usage,
    /// Delete a title's downloaded chapters; the title and its chapter list stay
    Delete {
        /// Manga id on the source (see `mangas list`)
        id: String,
        #[arg(long, default_value = "mangadex")]
        source: String,
    },
    /// Delete a cache the app rebuilds on its own
    Clear {
        #[arg(value_enum)]
        cache: Target,
    },
    /// Move chapters and detector models to another folder and use it from now on
    Move { dir: PathBuf },
}

#[derive(Clone, Copy, ValueEnum)]
enum Target {
    Covers,
    Models,
    All,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let folders = paths::folders()?;
    match args.command.unwrap_or(Command::Usage) {
        Command::Usage => print_usage(&storage::usage(&lib, &folders)?),
        Command::Delete { id, source } => {
            let deleted = storage::delete_downloads(&lib, &source, &id)?;
            outln!(
                "Deleted {} chapters of {id} ({}); the title stays in your library",
                deleted.chapters,
                human_size(deleted.bytes)
            )?;
            for path in deleted.kept {
                outln!("Kept {path}: it isn't in the download folder layout")?;
            }
            Ok(())
        }
        Command::Clear { cache } => {
            let caches = match cache {
                Target::Covers => vec![Cache::Covers],
                Target::Models => vec![Cache::Models],
                Target::All => vec![Cache::Covers, Cache::Models],
            };
            for cache in caches {
                let cleared = storage::clear_cache(&folders, cache)?;
                let name = match cache {
                    Cache::Covers => "cover cache",
                    Cache::Models => "detector models",
                };
                outln!("Cleared the {name}: {} freed", human_size(cleared.bytes))?;
            }
            Ok(())
        }
        Command::Move { dir } => {
            let moved = storage::move_data_dir(&lib, &folders.data_dir, &dir, &mut |_| {})?;
            outln!(
                "Moved {} files ({}) to {}",
                moved.files,
                human_size(moved.bytes),
                moved.data_dir
            )?;
            for path in moved.leftovers {
                outln!("Couldn't delete {path}; delete it by hand")?;
            }
            Ok(())
        }
    }
}

fn print_usage(usage: &Usage) -> anyhow::Result<()> {
    outln!("Data folder  {}", usage.data_dir)?;
    let rows = [
        ("Chapters", usage.chapters),
        ("Detector models", usage.models),
        ("Cover cache", usage.covers),
        ("Library and logs", usage.library + usage.logs),
    ];
    for (label, bytes) in rows {
        outln!("{label:<17}{:>10}", human_size(bytes))?;
    }
    if !usage.titles.is_empty() {
        outln!()?;
    }
    for t in &usage.titles {
        outln!(
            "{:>10}  {:>4} ch  {}  ({}:{})",
            human_size(t.bytes),
            t.chapters,
            t.title,
            t.source,
            t.manga_id
        )?;
    }
    Ok(())
}
