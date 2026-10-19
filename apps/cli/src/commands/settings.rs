use clap::{Args as ClapArgs, Subcommand};
use mangas_core::library::{self, Network, Settings};

use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Show or set the preferred chapter languages, most preferred first (shared with the app)
    Languages(LanguagesArgs),
    /// Show or change the Send to Kindle email settings (SMTP server, sender, Kindle address)
    Email(super::settings_email::EmailArgs),
    /// Show every shared setting; defaults are marked
    Show(super::settings_show::ShowArgs),
    /// Show or set the proxy for provider requests: system, none or an http(s) URL
    Proxy(ProxyArgs),
    /// Show or set how many chapters download at once (1-8)
    Concurrency(ConcurrencyArgs),
}

#[derive(ClapArgs)]
struct ProxyArgs {
    /// "system", "none" or a proxy URL like http://127.0.0.1:8888
    value: Option<String>,
}

#[derive(ClapArgs)]
struct ConcurrencyArgs {
    #[arg(value_parser = clap::value_parser!(u8).range(1..=8))]
    value: Option<u8>,
}

#[derive(ClapArgs)]
struct LanguagesArgs {
    /// Languages to save, e.g. "es-la,en"; prints the current ones when omitted
    list: Option<String>,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    match args.action {
        Action::Languages(args) => languages(args),
        Action::Email(args) => super::settings_email::run(args),
        Action::Show(args) => super::settings_show::run(args),
        Action::Proxy(args) => proxy(args),
        Action::Concurrency(args) => concurrency(args),
    }
}

/// Saves `edit` of the stored settings as a patch, so other fields changed meanwhile survive.
fn save(edit: impl Fn(&Settings) -> Settings) -> anyhow::Result<Settings> {
    let lib = library::open(&paths::library_db()?)?;
    let base = library::get_settings(&lib)?;
    Ok(library::edit_settings(&lib, Some(&base), &edit(&base))?)
}

fn proxy(args: ProxyArgs) -> anyhow::Result<()> {
    let settings = match args.value {
        Some(raw) => {
            let network = Network::parse(&raw)?;
            save(|s| network.store(s))?
        }
        None => crate::commands::stored_settings(),
    };
    let shown = match settings.network() {
        Network::System => "system".to_string(),
        Network::Direct => "none".to_string(),
        Network::Proxy(url) => mangas_providers::http::shown_proxy(&url),
    };
    outln!("{shown}")?;
    Ok(())
}

fn concurrency(args: ConcurrencyArgs) -> anyhow::Result<()> {
    let settings = match args.value {
        Some(n) => save(|s| Settings {
            download_concurrency: Some(n),
            ..s.clone()
        })?,
        None => crate::commands::stored_settings(),
    };
    outln!("{}", settings.concurrency())?;
    Ok(())
}

fn languages(args: LanguagesArgs) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let mut settings = library::get_settings(&lib)?;
    if let Some(list) = args.list {
        settings.preferred_languages = library::validate_languages(&list)?;
        library::set_settings(&lib, &settings)?;
    }
    outln!("{}", settings.languages().join(","))?;
    Ok(())
}
