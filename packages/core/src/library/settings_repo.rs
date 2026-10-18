use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::domain::{Error, Result};
use crate::export::Format;
use crate::library::settings_merge::merge_edit;
use crate::library::settings_prefs::{ProxyMode, Theme};
use crate::library::Library;
use crate::send::EmailSettings;
use crate::smartcut::Preset;

/// User preferences shared by the desktop app and the CLI. Every field is optional: `None`
/// means "use the built-in default", so new fields never need a migration. Defaults are left
/// out of the stored JSON, which only holds what the user changed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Root for downloaded chapters and detector models; defaults to the platform data dir.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_preset: Option<Preset>,
    /// Export format picked last; `None` follows the device profile's recommendation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_format: Option<Format>,
    /// Browser sessions captured for providers behind a verification wall, by source name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub provider_sessions: BTreeMap<String, ProviderSession>,
    /// Chapter languages in order of preference (ISO 639-1 with regional suffixes, "pt-br");
    /// empty means [`DEFAULT_LANGUAGE`](super::DEFAULT_LANGUAGE). See [`Settings::languages`].
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub preferred_languages: Vec<String>,
    /// Provider-specific options by provider id. The core does not know their shape; each
    /// provider deserialises its own entry (with defaults for anything missing).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub provider_options: BTreeMap<String, serde_json::Value>,
    /// Providers switched on or off, by id. A provider missing here is on, so providers added
    /// in a later release start enabled. See [`Settings::provider_enabled`].
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub providers_enabled: BTreeMap<String, bool>,
    /// Chapter languages per provider id, overriding `preferred_languages` for that provider
    /// when not empty. See [`Settings::languages_for`].
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub provider_languages: BTreeMap<String, Vec<String>>,
    /// Hours between background update checks; `None` is [`DEFAULT_UPDATE_HOURS`](super::DEFAULT_UPDATE_HOURS), 0 is off.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_check_hours: Option<u32>,
    /// Desktop notification when a background check finds new chapters; on when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_notifications: Option<bool>,
    /// Send to Kindle by email (SMTP server, sender, Kindle address).
    #[serde(skip_serializing_if = "is_default")]
    pub kindle_email: EmailSettings,
    /// Desktop colour scheme; `None` follows the OS ([`Theme::System`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
    /// Chapters downloading at once; `None` is [`DEFAULT_CONCURRENCY`](super::DEFAULT_CONCURRENCY). See
    /// [`Settings::concurrency`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_concurrency: Option<u8>,
    /// How provider requests reach the network; `None` is [`ProxyMode::System`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_mode: Option<ProxyMode>,
    /// The proxy used in [`ProxyMode::Custom`]; kept while another mode is picked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
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
    read(&*lib.conn()?)
}

fn read(conn: &Connection) -> Result<Settings> {
    let raw: Option<String> = conn
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
    write(&*lib.conn()?, settings)
}

fn write(conn: &Connection, settings: &Settings) -> Result<()> {
    conn.execute(
        "INSERT INTO setting (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        (KEY, encode(settings)?),
    )?;
    Ok(())
}

/// Saves an edit made on a copy of the settings (`base`, `None` = replace everything) over
/// whatever is stored now, in one write transaction so another process can't slip in
/// between; see [`merge_edit`]. Returns what is stored afterwards.
pub fn edit_settings(
    lib: &Library,
    base: Option<&Settings>,
    edited: &Settings,
) -> Result<Settings> {
    let mut conn = lib.conn()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current = read(&tx)?;
    let merged = match base {
        Some(base) => merge_edit(base, &current, edited)?,
        None => Settings {
            provider_sessions: current.provider_sessions,
            ..edited.clone()
        },
    };
    write(&tx, &merged)?;
    tx.commit()?;
    Ok(merged)
}
