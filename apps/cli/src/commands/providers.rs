use clap::{Args as ClapArgs, Subcommand};
use mangas_core::library::{self, Settings};
use mangas_providers::{doctor, options, ProviderInfo, ProviderStatus};

use super::stored_settings;
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    /// Walk one provider end to end: search, manga, chapters, pages and the first page image
    Check(CheckArgs),
    /// Show a provider's options, or change them with --set (shared with the desktop app)
    Options(OptionsArgs),
}

#[derive(ClapArgs)]
struct CheckArgs {
    /// Provider id (see `mangas providers`)
    id: String,
    /// Search text; the first results with chapters are used
    #[arg(long, default_value = "love")]
    query: String,
    /// Languages to try in order, e.g. "es,en"; the first with chapters is walked [default: your
    /// preferred ones]
    #[arg(long)]
    lang: Option<String>,
}

#[derive(ClapArgs)]
struct OptionsArgs {
    /// Provider id (see `mangas providers`)
    id: String,
    /// Change one option, e.g. `--set data_saver=true --set preferred_groups="A,B"`
    #[arg(long = "set", value_name = "KEY=VALUE")]
    set: Vec<String>,
    /// Forget the stored options and go back to the provider's defaults
    #[arg(long, conflicts_with = "set")]
    reset: bool,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    match args.action {
        None => list().await,
        Some(Action::Check(args)) => check(args).await,
        Some(Action::Options(args)) => options(args),
    }
}

/// Lists the providers with their status: one cheap request each, using the browser session
/// the desktop app stored (connect from Settings in the app).
async fn list() -> anyhow::Result<()> {
    for (info, status) in mangas_providers::statuses(&stored_settings()).await {
        outln!("{}", line(&info, &status))?;
    }
    Ok(())
}

/// Prints one line per step and fails (non-zero exit) unless every step passed.
async fn check(args: CheckArgs) -> anyhow::Result<()> {
    let settings = stored_settings();
    let requested = library::requested_languages(args.lang.as_deref(), &settings);
    let languages = mangas_providers::languages_for(&args.id, &requested);
    let report = doctor::first_language(&languages, |lang| {
        // Search filters by the preferred languages too, so each walk looks for `lang` alone.
        let settings = Settings {
            preferred_languages: vec![lang.clone()],
            ..settings.clone()
        };
        let (id, query) = (args.id.clone(), args.query.clone());
        async move {
            let source = mangas_providers::source(&id, &settings)?;
            anyhow::Ok(doctor::check(&*source, &query, &lang, &settings.network()).await)
        }
    })
    .await?;
    for line in doctor::lines(&report) {
        outln!("{line}")?;
    }
    anyhow::ensure!(report.passed(), "{} check failed", args.id);
    Ok(())
}

/// Prints the effective options as `key = value`, after applying `--set` / `--reset`.
fn options(args: OptionsArgs) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let mut settings = library::get_settings(&lib)?;
    if args.reset {
        settings = options::reset_options(&args.id, &settings)?;
        library::set_settings(&lib, &settings)?;
    } else if !args.set.is_empty() {
        settings = options::set_options(&args.id, &settings, &args.set)?;
        library::set_settings(&lib, &settings)?;
    }
    let current = options::options(&args.id, &settings)?;
    if current.is_empty() {
        outln!("{} has no options", args.id)?;
    }
    for (key, value) in &current {
        outln!("{key} = {}", options::display_value(value))?;
    }
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
