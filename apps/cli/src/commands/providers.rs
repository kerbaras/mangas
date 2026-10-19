use std::collections::HashMap;

use clap::{Args as ClapArgs, Subcommand};
use mangas_core::library::{self, Settings};
use mangas_core::sources::all_unreachable;
use mangas_providers::{doctor, options, ProviderInfo, ProviderStatus};
use serde::Serialize;

use super::stored_settings;
use crate::output::{outln, print_json};
use crate::{paths, Exit};

#[derive(ClapArgs)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Args {
    #[command(subcommand)]
    action: Option<Action>,
    /// Print one JSON object: `providers` (id, name, site, state, reason) and `all_unreachable`
    #[arg(long)]
    json: bool,
    /// Probe a provider at another base URL (mock servers in tests), e.g. mangadex=http://host
    #[arg(long = "base", value_name = "ID=URL", hide = true)]
    bases: Vec<String>,
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
        None => list(args.json, &args.bases).await,
        Some(Action::Check(args)) => check(args).await,
        Some(Action::Options(args)) => options(args),
    }
}

#[derive(Serialize)]
struct Row<'a> {
    id: &'a str,
    name: &'a str,
    site: &'a str,
    #[serde(flatten)]
    status: &'a ProviderStatus,
}

#[derive(Serialize)]
struct List<'a> {
    providers: Vec<Row<'a>>,
    all_unreachable: bool,
}

/// `--base ID=URL` pairs, refusing an unknown id rather than probing the live site instead.
fn parse_bases(bases: &[String]) -> anyhow::Result<HashMap<String, String>> {
    let known = mangas_providers::providers();
    bases
        .iter()
        .map(|b| match b.split_once('=') {
            Some((id, url)) if known.iter().any(|p| p.id == id) => {
                Ok((id.to_string(), url.to_string()))
            }
            Some((id, _)) => anyhow::bail!("--base: unknown provider {id:?}"),
            None => anyhow::bail!("--base takes ID=URL"),
        })
        .collect()
}

/// Lists the providers with their status: one cheap request each, using the browser session
/// the desktop app stored (connect from Settings in the app). Fails when none is reachable.
async fn list(json: bool, bases: &[String]) -> anyhow::Result<()> {
    let bases = parse_bases(bases)?;
    let statuses = mangas_providers::statuses_at(&stored_settings(), &bases).await;
    let all_unreachable = all_unreachable(statuses.iter().map(|(_, s)| s));
    if json {
        let providers = statuses.iter().map(|(i, status)| Row {
            id: i.id,
            name: i.name,
            site: i.site,
            status,
        });
        let providers = providers.collect();
        print_json(&List {
            providers,
            all_unreachable,
        })?;
    } else {
        for (info, status) in &statuses {
            outln!("{}", line(info, status))?;
        }
    }
    match all_unreachable {
        true => Err(Exit {
            code: 1,
            message: "no provider is reachable: check the network, proxy and TLS certificates"
                .into(),
        }
        .into()),
        false => Ok(()),
    }
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
