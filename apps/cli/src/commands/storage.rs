use std::path::PathBuf;

use clap::{Args as ClapArgs, Subcommand, ValueEnum};
use mangas_core::domain::MangaId;
use mangas_core::library;
use mangas_core::storage::{self, human_size, Cache, Usage};

use super::followed::followed_source;
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show how much space each folder and each title takes, and what an interrupted move left
    /// behind (the default)
    #[command(visible_alias = "status")]
    Usage,
    /// Delete a title's downloaded chapters; the title and its chapter list stay
    Delete {
        /// Manga id on the source (see `mangas list`)
        id: String,
        /// Source of the title [default: the one your library follows it on]
        #[arg(long)]
        source: Option<String>,
    },
    /// Delete a cache the app rebuilds on its own
    Clear {
        #[arg(value_enum)]
        cache: Target,
    },
    /// Move chapters and detector models to another folder and use it from now on
    Move {
        dir: PathBuf,
        /// Reuse DIR when it holds a partial copy an interrupted move left: everything in its
        /// chapters/ and models/ is deleted (files added there by hand too), then the move runs
        #[arg(long)]
        force: bool,
    },
    /// Delete the partial copy an interrupted move left (see `mangas storage status`)
    RemovePartial,
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
            let source = followed_source(&lib, &MangaId(id.clone()), source.as_deref())?;
            // Not during a data-folder move, which would keep copies of what this deletes; the
            // folders are read again under the lease, as a move may have just switched them.
            let _deleting = storage::hold_for_export(paths::data_dir)?;
            let chapters = paths::folders()?.chapters();
            let deleted = storage::delete_downloads(&lib, &chapters, &source, &id)?;
            outln!(
                "Deleted {} of {id} ({}); the title stays in your library",
                super::plural(deleted.chapters, "chapter", "chapters"),
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
        Command::Move { dir, force } => {
            let from = &folders.data_dir;
            let moved = storage::move_data_dir_with(&lib, from, &dir, force, &mut |_| {})?;
            outln!(
                "Moved {} ({}) to {}",
                super::plural(moved.files, "file", "files"),
                human_size(moved.bytes),
                moved.data_dir
            )?;
            for path in moved.leftovers {
                outln!("Couldn't delete {path}; delete it by hand")?;
            }
            Ok(())
        }
        Command::RemovePartial => {
            match storage::remove_leftover_copy(&lib, &folders.data_dir)? {
                Some(copy) => outln!(
                    "Removed the partial copy in {} ({})",
                    copy.path,
                    human_size(copy.bytes)
                )?,
                None => outln!("No interrupted move left a partial copy")?,
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
    if let Some(copy) = &usage.partial_move {
        outln!(
            "\nPartial copy   {} ({}), left by an interrupted move\n  \
             remove it: mangas storage remove-partial; or reuse it, deleting everything in its chapters/ and models/: mangas storage move --force <dir>",
            copy.path,
            human_size(copy.bytes)
        )?;
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
