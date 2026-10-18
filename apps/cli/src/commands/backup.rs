use std::path::PathBuf;

use clap::{Args as ClapArgs, Subcommand};
use mangas_core::backup::{self, Preferences};
use mangas_core::library;

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
    /// Follow the titles in a backup or a HaruNeko bookmarks export. Its preferences replace
    /// yours only when the library is empty, unless a flag says otherwise
    Import {
        file: PathBuf,
        /// Keep your current preferences even if the library is empty
        #[arg(long, conflicts_with = "backup_preferences")]
        keep_preferences: bool,
        /// Take the backup's preferences even if the library already has titles
        #[arg(long)]
        backup_preferences: bool,
    },
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
        Command::Import {
            file,
            keep_preferences,
            backup_preferences,
        } => {
            let known: Vec<_> = mangas_providers::providers().iter().map(|p| p.id).collect();
            let preferences = match (keep_preferences, backup_preferences) {
                (true, _) => Preferences::Keep,
                (_, true) => Preferences::Backup,
                _ => Preferences::Auto,
            };
            let report = backup::import_with(&lib, &file, &known, preferences)?;
            outln!(
                "Followed {} ({} already in your library)",
                super::plural(report.added, "new title", "new titles"),
                report.already
            )?;
            if report.settings {
                outln!("Restored preferences (provider sessions and the data folder were kept)")?;
            }
            if report.kept_settings {
                outln!("Kept your preferences (--backup-preferences takes the backup's)")?;
            }
            for skipped in report.skipped {
                outln!("Skipped {}: {}", skipped.title, skipped.reason)?;
            }
        }
    }
    Ok(())
}
