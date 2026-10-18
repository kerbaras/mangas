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
}

/// Chapter language used when no preference is stored.
pub const DEFAULT_LANGUAGE: &str = "en";

impl Settings {
    /// Preferred chapter languages, never empty.
    pub fn languages(&self) -> Vec<String> {
        let set: Vec<String> = self
            .preferred_languages
            .iter()
            .map(|l| l.trim().to_ascii_lowercase())
            .filter(|l| !l.is_empty())
            .collect();
        if set.is_empty() {
            vec![DEFAULT_LANGUAGE.to_string()]
        } else {
            set
        }
    }
}

/// Cookies and the user agent that obtained them; the HTTP client must send both unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderSession {
    pub cookies: String,
    pub user_agent: String,
    pub saved_at: Option<String>,
}

const KEY: &str = "app";

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

pub fn set_settings(lib: &Library, settings: &Settings) -> Result<()> {
    let json = serde_json::to_string(settings).map_err(json_error)?;
    lib.conn()?.execute(
        "INSERT INTO setting (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        (KEY, json),
    )?;
    Ok(())
}
