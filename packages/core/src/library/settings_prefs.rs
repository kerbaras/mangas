//! Typed views of [`Settings`]: built-in defaults and normalisation, no storage.

use serde::{Deserialize, Serialize};

use super::Settings;
use crate::domain::{Error, Result};

/// Hours between background update checks when none is stored.
pub const DEFAULT_UPDATE_HOURS: u32 = 6;

/// Chapter language used when no preference is stored.
pub const DEFAULT_LANGUAGE: &str = "en";

/// Chapters downloading at once when none is stored.
pub const DEFAULT_CONCURRENCY: u8 = 4;

/// The range Settings offers for [`Settings::download_concurrency`].
pub const CONCURRENCY_RANGE: std::ops::RangeInclusive<u8> = 1..=8;

/// Desktop colour scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    /// Follows the OS (`prefers-color-scheme`).
    #[default]
    System,
}

/// Where the proxy for provider requests comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// The OS proxy (`HTTP(S)_PROXY`, `ALL_PROXY`; the system settings on macOS and Windows).
    #[default]
    System,
    /// Straight to the sites, ignoring any system proxy.
    None,
    /// [`Settings::proxy_url`].
    Custom,
}

/// How provider requests reach the network, resolved from the settings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Network {
    #[default]
    System,
    Direct,
    /// An `http://` or `https://` proxy URL.
    Proxy(String),
}

impl Settings {
    /// Preferred chapter languages, never empty.
    pub fn languages(&self) -> Vec<String> {
        let set = normalise(&self.preferred_languages);
        if set.is_empty() {
            vec![DEFAULT_LANGUAGE.to_string()]
        } else {
            set
        }
    }

    /// Chapter languages for `provider`: its own list when one is stored, else [`Self::languages`].
    pub fn languages_for(&self, provider: &str) -> Vec<String> {
        let own = self
            .provider_languages
            .get(provider)
            .map(|l| normalise(l))
            .unwrap_or_default();
        if own.is_empty() {
            self.languages()
        } else {
            own
        }
    }

    /// Hours between background update checks; 0 means they are off.
    pub fn update_hours(&self) -> u32 {
        self.update_check_hours.unwrap_or(DEFAULT_UPDATE_HOURS)
    }

    /// Whether background checks that find new chapters notify the desktop.
    pub fn notify_updates(&self) -> bool {
        self.update_notifications.unwrap_or(true)
    }

    /// Whether `provider` takes part in searches; on unless switched off.
    pub fn provider_enabled(&self, provider: &str) -> bool {
        self.providers_enabled
            .get(provider)
            .copied()
            .unwrap_or(true)
    }

    /// The groups a provider prefers when a chapter has several releases
    /// (`provider_options[source].preferred_groups`, first = best).
    pub fn preferred_groups(&self, source: &str) -> Vec<String> {
        self.provider_options
            .get(source)
            .and_then(|o| o.get("preferred_groups"))
            .and_then(|g| g.as_array())
            .map(|g| {
                g.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Chapters downloading at once, within [`CONCURRENCY_RANGE`].
    pub fn concurrency(&self) -> u8 {
        let n = self.download_concurrency.unwrap_or(DEFAULT_CONCURRENCY);
        n.clamp(*CONCURRENCY_RANGE.start(), *CONCURRENCY_RANGE.end())
    }

    /// How provider requests reach the network. A custom mode without a usable URL falls back
    /// to the system proxy rather than failing every request.
    pub fn network(&self) -> Network {
        match self.proxy_mode.unwrap_or_default() {
            ProxyMode::System => Network::System,
            ProxyMode::None => Network::Direct,
            ProxyMode::Custom => self
                .proxy_url
                .as_deref()
                .and_then(|u| proxy_url(u).ok())
                .map_or(Network::System, Network::Proxy),
        }
    }
}

impl Network {
    /// `system`, `none` (or `direct`) or a proxy URL, as typed on the command line.
    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "system" => Ok(Network::System),
            "none" | "direct" => Ok(Network::Direct),
            _ => proxy_url(raw).map(Network::Proxy),
        }
    }

    /// The settings that select this network; a custom proxy keeps `previous` URL otherwise.
    pub fn store(&self, settings: &Settings) -> Settings {
        let (mode, url) = match self {
            Network::System => (None, settings.proxy_url.clone()),
            Network::Direct => (Some(ProxyMode::None), settings.proxy_url.clone()),
            Network::Proxy(url) => (Some(ProxyMode::Custom), Some(url.clone())),
        };
        Settings {
            proxy_mode: mode,
            proxy_url: url,
            ..settings.clone()
        }
    }
}

/// `raw` as a proxy URL: trimmed, `http` or `https` with a host. A bare `host:port` gets
/// `http://`, the scheme most local proxies speak.
pub fn proxy_url(raw: &str) -> Result<String> {
    let raw = raw.trim();
    let full = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("http://{raw}")
    };
    let invalid = |why: &str| Error::Other(format!("invalid proxy URL {raw:?}: {why}"));
    let parsed = url::Url::parse(&full).map_err(|e| invalid(&e.to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(invalid("use http:// or https://"));
    }
    if parsed.host_str().is_none_or(str::is_empty) {
        return Err(invalid("missing host"));
    }
    Ok(full)
}

fn normalise(languages: &[String]) -> Vec<String> {
    languages
        .iter()
        .map(|l| l.trim().to_ascii_lowercase())
        .filter(|l| !l.is_empty())
        .collect()
}
