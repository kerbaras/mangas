//! `mangas` command line. Each subcommand lives in `commands/` and only parses input, calls
//! `mangas_core`, and prints the result. The desktop binary links this crate so one executable
//! serves both: with arguments it runs a command, without it opens the app.

mod commands;
mod exit;
mod output;
mod paths;

use std::ffi::OsString;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mangas", version, about = "Manga library and Kindle exporter")]
pub struct Cli {
    /// Log more to stderr: -v info, -vv debug (MANGAS_LOG overrides)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
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
    /// Show a manga's details: credits, year, tags and description
    Info(commands::info::Args),
    /// List the chapters of a manga
    Chapters(commands::chapters::Args),
    /// Download chapters of a followed manga
    Download(commands::download::Args),
    /// Show and control the download queue shared with the desktop app
    Queue(commands::queue::Args),
    /// Check followed manga for new chapters
    Update(commands::update::Args),
    /// Import chapter folders or .cbz files from disk into the library
    Import(commands::import::Args),
    /// Export downloaded chapters as a Kindle-ready EPUB
    Kindle(commands::kindle::Args),
    /// List the providers and whether they are ready to use
    Providers(commands::providers::Args),
    /// Show or change app settings shared with the desktop app
    Settings(commands::settings::Args),
    /// Show disk usage, delete downloads, clear caches or move the data folder
    Storage(commands::storage::Args),
    /// Export or import a library backup (followed titles and preferences)
    Backup(commands::backup::Args),
}

/// Parses `args` (including the program name) and runs the command. Exits on `--help`,
/// `--version` and usage errors, like any clap binary.
pub use exit::{exit_code, Exit};

pub async fn run<I, T>(args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    // Already initialised when embedded in a host that set up its own logging; keep that one.
    let _ = mangas_core::logging::init_stderr(cli.verbose);
    if let Ok(dir) = paths::rate_limits() {
        mangas_providers::HostLimiters::shared().share_across_processes(dir);
    }
    output::ignore_broken_pipe(dispatch(cli.command).await)
}

async fn dispatch(command: Command) -> anyhow::Result<()> {
    match command {
        Command::List(args) => commands::list::run(args),
        Command::Search(args) => commands::search::run(args).await,
        Command::Add(args) => commands::add::run(args).await,
        Command::Remove(args) => commands::remove::run(args),
        Command::Info(args) => commands::info::run(args).await,
        Command::Chapters(args) => commands::chapters::run(args).await,
        Command::Download(args) => commands::download::run(args).await,
        Command::Queue(args) => commands::queue::run(args).await,
        Command::Update(args) => commands::update::run(args).await,
        Command::Import(args) => commands::import::run(args),
        Command::Kindle(args) => commands::kindle::run(args).await,
        Command::Providers(args) => commands::providers::run(args).await,
        Command::Settings(args) => commands::settings::run(args),
        Command::Storage(args) => commands::storage::run(args),
        Command::Backup(args) => commands::backup::run(args),
    }
}
