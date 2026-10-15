//! `mangas` CLI. Each subcommand lives in `commands/` and only parses input, calls
//! `mangas_core`, and prints the result.

mod commands;
mod paths;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mangas", version, about = "Manga library and Kindle exporter")]
struct Cli {
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
    /// Download chapters of a followed manga
    Download(commands::download::Args),
    /// Export downloaded chapters as a Kindle-ready EPUB
    Kindle(commands::kindle::Args),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::List(args) => commands::list::run(args),
        Command::Search(args) => commands::search::run(args).await,
        Command::Add(args) => commands::add::run(args).await,
        Command::Download(args) => commands::download::run(args).await,
        Command::Kindle(args) => commands::kindle::run(args).await,
    }
}
