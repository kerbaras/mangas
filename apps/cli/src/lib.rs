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
#[command(
    name = "mangas",
    version,
    about = "Manga library and e-reader exporter"
)]
pub struct Cli {
    /// Log more to stderr: -v info, -vv debug (MANGAS_LOG overrides)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
    /// Proxy for provider requests this run: system, none or an http(s) URL (overrides Settings)
    #[arg(long, global = true, value_name = "URL")]
    proxy: Option<String>,
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
    /// Show a followed manga's scanlation groups or narrow it to one
    Group(commands::group::Args),
    /// Show and control the download queue shared with the desktop app
    Queue(commands::queue::Args),
    /// Check followed manga for new chapters
    Update(commands::update::Args),
    /// Import chapter folders or .cbz files from disk into the library
    Import(commands::import::Args),
    /// Export chapter folders as one EPUB, AZW3, CBZ or PDF for an e-reader profile (Kindle EPUB by
    /// default, AZW3 with --send usb), optionally sending it (--send)
    Kindle(commands::kindle::Args),
    /// Export a followed manga's downloaded chapters as volume books: EPUB, AZW3, CBZ or PDF (TOC, series, cover)
    Export(commands::export::Args),
    /// Send a book to the Kindle: an EPUB or PDF by email, an AZW3 or PDF over USB
    Send(commands::send::Args),
    /// List the providers and whether they are ready to use. Exits 1 when every provider is
    /// unreachable (no network, a proxy or TLS certificates problem); a partial outage exits 0
    Providers(commands::providers::Args),
    /// Show or change app settings shared with the desktop app
    Settings(commands::settings::Args),
    /// Show disk usage, delete downloads, clear caches or move the data folder
    Storage(commands::storage::Args),
    /// Export or import a library backup (followed titles and preferences)
    Backup(commands::backup::Args),
    /// Show, download or remove detector support (ONNX Runtime and models) for smart cutting
    Ml(commands::ml::Args),
}

/// Parses `args` (including the program name) and runs the command. Exits on `--help`,
/// `--version` and usage errors, like any clap binary.
pub use exit::{describe, exit_code, Exit};

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
    if let Some(raw) = &cli.proxy {
        match mangas_core::library::Network::parse(raw) {
            Ok(network) => {
                mangas_providers::http::override_network(network);
            }
            Err(e) => return output::ignore_broken_pipe(Err(e.into())),
        }
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
        Command::Group(args) => commands::group::run(args),
        Command::Queue(args) => commands::queue::run(args).await,
        Command::Update(args) => commands::update::run(args).await,
        Command::Import(args) => commands::import::run(args),
        Command::Kindle(args) => commands::kindle::run(args).await,
        Command::Export(args) => commands::export::run(args).await,
        Command::Send(args) => commands::send::run(args).await,
        Command::Providers(args) => commands::providers::run(args).await,
        Command::Settings(args) => commands::settings::run(args),
        Command::Storage(args) => commands::storage::run(args),
        Command::Backup(args) => commands::backup::run(args),
        Command::Ml(args) => commands::ml::run(args).await,
    }
}
