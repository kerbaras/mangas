//! `mangas settings show`: every preference the app and the CLI share, defaults marked.

use clap::Args as ClapArgs;
use mangas_core::library::{self, Network, Settings, Theme};

use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct ShowArgs {
    /// Print the stored JSON (only what was changed; sessions and the SMTP password left out)
    #[arg(long)]
    json: bool,
}

pub fn run(args: ShowArgs) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let settings = library::get_settings(&lib)?;
    if args.json {
        outln!("{}", serde_json::to_string_pretty(&shareable(&settings))?)?;
        return Ok(());
    }
    for (key, value) in lines(&settings) {
        outln!("{key:<22}{value}")?;
    }
    Ok(())
}

fn shareable(settings: &Settings) -> Settings {
    let mut out = Settings {
        provider_sessions: Default::default(),
        ..settings.clone()
    };
    if !out.kindle_email.password.is_empty() {
        out.kindle_email.password = "***".into();
    }
    out
}

fn or_default<T>(value: Option<T>, shown: impl Fn(T) -> String, default: &str) -> String {
    value.map_or(format!("{default} (default)"), shown)
}

fn lines(s: &Settings) -> Vec<(&'static str, String)> {
    let theme = |t: Theme| format!("{t:?}").to_lowercase();
    let ratings = mangas_providers::options::options("mangadex", s)
        .ok()
        .and_then(|o| o.get("content_ratings").cloned())
        .and_then(|r| serde_json::from_value::<Vec<String>>(r).ok())
        .map_or("-".into(), |r| r.join(","));
    let languages = if s.preferred_languages.is_empty() {
        format!("{} (default)", s.languages().join(","))
    } else {
        s.languages().join(",")
    };
    let hours = match s.update_hours() {
        0 => "off".to_string(),
        h => format!("every {h} h"),
    };
    vec![
        ("theme", or_default(s.theme, theme, "system")),
        (
            "download concurrency",
            or_default(s.download_concurrency, |_| s.concurrency().to_string(), "4"),
        ),
        ("languages", languages),
        ("content rating", format!("{ratings} (mangadex)")),
        ("proxy", proxy(s)),
        (
            "data folder",
            or_default(s.data_dir.clone(), |d| d, "platform data dir"),
        ),
        (
            "export device",
            or_default(s.default_device.clone(), |d| d, "kindle-paperwhite12"),
        ),
        (
            "export format",
            or_default(
                s.default_format,
                |f| format!("{f:?}").to_lowercase(),
                "device's",
            ),
        ),
        (
            "smart cut preset",
            or_default(s.default_preset, |p| format!("{p:?}"), "balanced"),
        ),
        ("update checks", hours),
        ("notifications", on_off(s.notify_updates())),
        ("kindle email", or_dash(&s.kindle_email.to)),
    ]
}

fn proxy(s: &Settings) -> String {
    let url = s
        .proxy_url
        .as_deref()
        .map(mangas_providers::http::shown_proxy);
    match (s.proxy_mode, s.network()) {
        (None, _) => "system (default)".into(),
        (_, Network::System) => format!("system (custom URL unusable: {})", or_dash_opt(url)),
        (_, Network::Direct) => "none".into(),
        (_, Network::Proxy(u)) => mangas_providers::http::shown_proxy(&u),
    }
}

fn on_off(on: bool) -> String {
    if on { "on" } else { "off" }.to_string()
}

fn or_dash(value: &str) -> String {
    if value.is_empty() { "-" } else { value }.to_string()
}

fn or_dash_opt(value: Option<String>) -> String {
    value.unwrap_or_else(|| "-".into())
}
