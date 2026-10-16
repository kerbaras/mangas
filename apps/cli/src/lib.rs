//! `mangas` command line. Each subcommand lives in `commands/` and only parses input, calls
//! `mangas_core`, and prints the result. The desktop binary links this crate so one executable
//! serves both: with arguments it runs a command, without it opens the app.

mod commands;
mod paths;

use std::ffi::OsString;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mangas", version, about = "Manga library and Kindle exporter")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the manga in your library
    List(commands::list::Args),
    /// Search a source for manga
    Search(commands::search::Args),
    /// Follow a manga (add it to the library)
    Add(commands::add::Args),
    /// Unfollow a manga (remove it from the library)
    Remove(commands::remove::Args),
    /// List the chapters of a manga
    Chapters(commands::chapters::Args),
    /// Download chapters of a followed manga
    Download(commands::download::Args),
    /// Export downloaded chapters as a Kindle-ready EPUB
    Kindle(commands::kindle::Args),
}

/// Parses `args` (including the program name) and runs the command. Exits on `--help`,
/// `--version` and usage errors, like any clap binary.
pub async fn run<I, T>(args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match Cli::parse_from(args).command {
        Command::List(args) => commands::list::run(args),
        Command::Search(args) => commands::search::run(args).await,
        Command::Add(args) => commands::add::run(args).await,
        Command::Remove(args) => commands::remove::run(args),
        Command::Chapters(args) => commands::chapters::run(args).await,
        Command::Download(args) => commands::download::run(args).await,
        Command::Kindle(args) => commands::kindle::run(args).await,
    }
}
