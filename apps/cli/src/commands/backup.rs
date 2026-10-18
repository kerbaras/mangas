use std::path::PathBuf;

use clap::{Args as ClapArgs, Subcommand};
use mangas_core::{backup, library};

use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write followed titles, categories, reading status, read marks and preferences to a JSON file
    Export { file: PathBuf },
    /// Follow the titles in a backup or a HaruNeko bookmarks export and apply its preferences
    Import { file: PathBuf },
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    match args.command {
        Command::Export { file } => {
            let summary = backup::export_to(&lib, &file, env!("CARGO_PKG_VERSION"))?;
            outln!(
                "Exported {} to {}",
                super::plural(summary.titles, "title", "titles"),
                file.display()
            )?;
            if summary.skipped_local > 0 {
                outln!(
                    "Left out {}: their files aren't part of a backup",
                    super::plural(summary.skipped_local, "imported title", "imported titles")
                )?;
            }
        }
        Command::Import { file } => {
            let known: Vec<_> = mangas_providers::providers().iter().map(|p| p.id).collect();
            let report = backup::import_from(&lib, &file, &known)?;
            outln!(
                "Followed {} ({} already in your library)",
                super::plural(report.added, "new title", "new titles"),
                report.already
            )?;
            if report.settings {
                outln!("Restored preferences (provider sessions and the data folder were kept)")?;
            }
            for skipped in report.skipped {
                outln!("Skipped {}: {}", skipped.title, skipped.reason)?;
            }
        }
    }
    Ok(())
}
