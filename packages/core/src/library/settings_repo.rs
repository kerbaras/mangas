use std::collections::BTreeMap;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::domain::{Error, Result};
use crate::library::Library;
use crate::smartcut::Preset;

/// User preferences shared by the desktop app and the CLI. Every field is optional: `None`
/// means "use the built-in default", so new fields never need a migration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Root for downloaded chapters and detector models; defaults to the platform data dir.
    pub data_dir: Option<String>,
    pub default_device: Option<String>,
    pub default_preset: Option<Preset>,
    /// Browser sessions captured for providers behind a verification wall, by source name.
    pub provider_sessions: BTreeMap<String, ProviderSession>,
    /// Chapter languages in order of preference (ISO 639-1 with regional suffixes, "pt-br");
    /// empty means [`DEFAULT_LANGUAGE`]. See [`Settings::languages`].
    pub preferred_languages: Vec<String>,
    /// Provider-specific options by provider id. The core does not know their shape; each
    /// provider deserialises its own entry (with defaults for anything missing).
    pub provider_options: BTreeMap<String, serde_json::Value>,
    /// Providers switched on or off, by id. A provider missing here is on, so providers added
    /// in a later release start enabled. See [`Settings::provider_enabled`].
    pub providers_enabled: BTreeMap<String, bool>,
    /// Chapter languages per provider id, overriding `preferred_languages` for that provider
    /// when not empty. See [`Settings::languages_for`].
    pub provider_languages: BTreeMap<String, Vec<String>>,
}

/// Chapter language used when no preference is stored.
pub const DEFAULT_LANGUAGE: &str = "en";

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

    /// Whether `provider` takes part in searches; on unless switched off.
    pub fn provider_enabled(&self, provider: &str) -> bool {
        self.providers_enabled
            .get(provider)
            .copied()
            .unwrap_or(true)
    }
}

fn normalise(languages: &[String]) -> Vec<String> {
    languages
        .iter()
        .map(|l| l.trim().to_ascii_lowercase())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Cookies and the user agent that obtained them; the HTTP client must send both unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderSession {
    pub cookies: String,
    pub user_agent: String,
    pub saved_at: Option<String>,
}

pub(crate) const KEY: &str = "app";

fn json_error(e: serde_json::Error) -> Error {
    Error::Other(format!("settings: {e}"))
}

pub fn get_settings(lib: &Library) -> Result<Settings> {
    let raw: Option<String> = lib
        .conn()?
        .query_row("SELECT value FROM setting WHERE key = ?1", [KEY], |r| {
            r.get(0)
        })
        .optional()?;
    raw.map_or(Ok(Settings::default()), |s| {
        serde_json::from_str(&s).map_err(json_error)
    })
}

pub(crate) fn encode(settings: &Settings) -> Result<String> {
    serde_json::to_string(settings).map_err(json_error)
}

pub fn set_settings(lib: &Library, settings: &Settings) -> Result<()> {
    let json = encode(settings)?;
    lib.conn()?.execute(
        "INSERT INTO setting (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        (KEY, json),
    )?;
    Ok(())
}
