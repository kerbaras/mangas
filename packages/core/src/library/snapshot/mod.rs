//! The user's own library state that a backup carries besides title metadata: categories and
//! their assignments, reading status, per-title flags and times, synced chapter languages, export
//! presets, and
//! the stored chapter rows with their exported (read) and first-seen marks. Times are kept as
//! the database text, so a restore writes back exactly what was read.

mod read;
mod write;

use serde::{Deserialize, Serialize};

use super::ReadingStatus;

pub use read::read_snapshot;
pub use write::restore_snapshot;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LibrarySnapshot {
    pub categories: Vec<CategoryState>,
    pub titles: Vec<TitleState>,
    pub chapters: Vec<ChapterState>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CategoryState {
    pub name: String,
    pub position: i64,
}

/// A followed title's user state; `categories` are names, so they survive new ids.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TitleState {
    pub source: String,
    pub id: String,
    pub added_at: Option<String>,
    pub updated_at: Option<String>,
    pub reading_status: ReadingStatus,
    pub auto_download: bool,
    pub last_viewed_at: Option<String>,
    pub categories: Vec<String>,
    pub synced_languages: Vec<String>,
    /// The series export preset as stored (JSON), if any.
    pub export_preset: Option<String>,
}

/// A stored chapter row minus `local_path`: downloaded files aren't part of a backup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapterState {
    pub source: String,
    pub manga_id: String,
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    pub language: String,
    #[serde(default)]
    pub volume: Option<String>,
    pub number: String,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default = "available")]
    pub availability: String,
    #[serde(default)]
    pub external_url: Option<String>,
    #[serde(default)]
    pub exported_at: Option<String>,
    #[serde(default)]
    pub first_seen_at: Option<String>,
}

fn available() -> String {
    "available".into()
}

impl LibrarySnapshot {
    /// The part of the snapshot about titles `keep` accepts (categories are always kept).
    pub fn only(self, keep: impl Fn(&str) -> bool) -> Self {
        Self {
            categories: self.categories,
            titles: self
                .titles
                .into_iter()
                .filter(|t| keep(&t.source))
                .collect(),
            chapters: self
                .chapters
                .into_iter()
                .filter(|c| keep(&c.source))
                .collect(),
        }
    }
}
