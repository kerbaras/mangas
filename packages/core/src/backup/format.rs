use serde::{Deserialize, Serialize};

use crate::domain::{Error, Manga, Result};
use crate::library::{LibrarySnapshot, Settings};

pub const FORMAT: &str = "mangas-backup";
/// Bumped when a field changes meaning; new optional fields don't need a bump.
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backup {
    pub format: String,
    pub version: u32,
    pub exported_at: String,
    pub app_version: String,
    /// One per followed title, in HaruNeko's exact bookmark shape (it rejects extra keys).
    pub bookmarks: Vec<Bookmark>,
    /// Full metadata of the bookmarked titles, so a restore shows covers and details at once.
    #[serde(default)]
    pub library: Vec<Manga>,
    /// Preferences without provider sessions or the data folder.
    #[serde(default)]
    pub settings: Option<Settings>,
    /// Categories, reading status, flags and chapter marks (top-level `categories`, `titles`,
    /// `chapters`). Backups from before 0.3.0 have none and restore with defaults.
    #[serde(flatten)]
    pub state: LibrarySnapshot,
}

/// HaruNeko's `BookmarkSerialized`: times are Unix milliseconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Bookmark {
    pub created: i64,
    pub updated: i64,
    pub title: String,
    pub media: EntryRef,
    pub info: TrackerRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryRef {
    #[serde(rename = "ProviderID")]
    pub provider_id: String,
    #[serde(rename = "EntryID")]
    pub entry_id: String,
}

/// HaruNeko's optional tracker link; mangas has no trackers, so it writes nulls.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackerRef {
    #[serde(rename = "ProviderID")]
    pub provider_id: Option<String>,
    #[serde(rename = "EntryID")]
    pub entry_id: Option<String>,
}

fn invalid(detail: impl std::fmt::Display) -> Error {
    Error::Import(format!(
        "this file isn't a mangas backup or a HaruNeko bookmarks export ({detail})"
    ))
}

/// Reads a mangas backup, or a bare HaruNeko bookmark list (which carries no settings).
pub fn parse(text: &str) -> Result<Backup> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(invalid)?;
    if value.is_array() {
        let bookmarks = serde_json::from_value(value).map_err(invalid)?;
        return Ok(Backup {
            format: FORMAT.into(),
            version: VERSION,
            exported_at: String::new(),
            app_version: String::new(),
            bookmarks,
            library: Vec::new(),
            settings: None,
            state: LibrarySnapshot::default(),
        });
    }
    if value.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(invalid("no \"format\": \"mangas-backup\" field"));
    }
    let backup: Backup = serde_json::from_value(value).map_err(invalid)?;
    if backup.version > VERSION {
        return Err(Error::Import(format!(
            "this backup was made by a newer version of mangas (format version {}); update the app to import it",
            backup.version
        )));
    }
    Ok(backup)
}
