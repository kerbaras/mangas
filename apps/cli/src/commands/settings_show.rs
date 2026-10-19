//! `mangas settings show`: every preference the app and the CLI share, defaults marked.

use clap::Args as ClapArgs;
use mangas_core::library::{self, Network, Settings, Theme, DEFAULT_UPDATE_HOURS};

use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct ShowArgs {
    /// Print the stored JSON (only what was changed; sessions left out, passwords masked)
    #[arg(long)]
    json: bool,
}

pub fn run(args: ShowArgs) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let settings = library::get_settings(&lib)?;
    if args.json {
        let defaults = serde_json::to_value(Settings::default())?;
        let diff = changed(serde_json::to_value(shareable(&settings))?, &defaults);
        outln!("{}", serde_json::to_string_pretty(&diff)?)?;
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
        proxy_url: settings
            .proxy_url
            .as_deref()
            .map(mangas_providers::http::shown_proxy),
        ..settings.clone()
    };
    if !out.kindle_email.password.is_empty() {
        out.kindle_email.password = "***".into();
    }
    out
}

/// The entries of `value` that differ from `default`, at every depth.
fn changed(value: serde_json::Value, default: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter_map(|(k, v)| {
                    let base = default.get(&k).unwrap_or(&Value::Null);
                    let v = changed(v, base);
                    let unset = v.is_null() || v.as_object().is_some_and(|o| o.is_empty());
                    (!unset && &v != base).then_some((k, v))
                })
                .collect(),
        ),
        other => other,
    }
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
    let rated = s
        .provider_options
        .get("mangadex")
        .is_some_and(|o| o.get("content_ratings").is_some());
    let ratings = if rated {
        format!("{ratings} (mangadex)")
    } else {
        format!("{ratings} (mangadex, default)")
    };
    let languages = if s.preferred_languages.is_empty() {
        format!("{} (default)", s.languages().join(","))
    } else {
        s.languages().join(",")
    };
    let hours = |h: u32| match h {
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
        ("content rating", ratings),
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
            or_default(s.default_preset, kebab, "balanced"),
        ),
        (
            "update checks",
            or_default(s.update_check_hours, hours, &hours(DEFAULT_UPDATE_HOURS)),
        ),
        (
            "notifications",
            or_default(s.update_notifications, on_off, "on"),
        ),
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
    if value.is_empty() {
        "- (default)"
    } else {
        value
    }
    .to_string()
}

/// How the CLI flags spell `value` (its serde name, e.g. `fewest-cuts`).
fn kebab(value: impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

fn or_dash_opt(value: Option<String>) -> String {
    value.unwrap_or_else(|| "-".into())
}

#[cfg(test)]
mod tests {
    use mangas_core::smartcut::Preset;

    #[test]
    fn presets_read_as_the_cli_flag_spells_them() {
        assert_eq!(super::kebab(Preset::FewestCuts), "fewest-cuts");
        assert_eq!(super::kebab(Preset::NoShrink), "no-shrink");
    }
}
