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
