use clap::{Args as ClapArgs, Subcommand};
use mangas_providers::{doctor, ProviderInfo, ProviderStatus};

use super::{source_by_name, stored_settings};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    /// Walk one provider end to end: search, manga, chapters, pages and the first page image
    Check(CheckArgs),
}

#[derive(ClapArgs)]
struct CheckArgs {
    /// Provider id (see `mangas providers`)
    id: String,
    /// Search text; the first results with chapters are used
    #[arg(long, default_value = "love")]
    query: String,
    /// Chapter language
    #[arg(long, default_value = "en")]
    lang: String,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    match args.action {
        None => list().await,
        Some(Action::Check(args)) => check(args).await,
    }
}

/// Lists the providers with their status: one cheap request each, using the browser session
/// the desktop app stored (connect from Settings in the app).
async fn list() -> anyhow::Result<()> {
    for (info, status) in mangas_providers::statuses(&stored_settings()).await {
        println!("{}", line(&info, &status));
    }
    Ok(())
}

/// Prints one line per step and fails (non-zero exit) unless every step passed.
async fn check(args: CheckArgs) -> anyhow::Result<()> {
    let source = source_by_name(&args.id)?;
    let report = doctor::check(&*source, &args.query, &args.lang).await;
    for line in doctor::lines(&report) {
        println!("{line}");
    }
    anyhow::ensure!(report.passed(), "{} check failed", args.id);
    Ok(())
}

fn line(info: &ProviderInfo, status: &ProviderStatus) -> String {
    let hint = match status {
        ProviderStatus::NeedsSession => " (connect in the app: Settings > Providers)",
        _ => "",
    };
    format!(
        "{:<10} {:<10} {:<24} {status}{hint}",
        info.id, info.name, info.site
    )
}
