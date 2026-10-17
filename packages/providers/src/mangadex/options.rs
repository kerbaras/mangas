//! MangaDex preferences, stored as `Settings.provider_options["mangadex"]`.

use mangas_core::library::Settings;
use serde::{Deserialize, Serialize};

use super::mapping::SOURCE_NAME;

/// Every rating MangaDex knows, from safest.
pub const ALL_CONTENT_RATINGS: [&str; 4] = ["safe", "suggestive", "erotica", "pornographic"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MangaDexOptions {
    /// Ratings allowed in search results (`contentRating[]`).
    pub content_ratings: Vec<String>,
    /// Compressed pages (`data-saver`) instead of the originals.
    pub data_saver: bool,
    /// Group names that win when several uploads share a chapter number (first = best).
    pub preferred_groups: Vec<String>,
}

impl Default for MangaDexOptions {
    fn default() -> Self {
        Self {
            content_ratings: vec!["safe".into(), "suggestive".into()],
            data_saver: false,
            preferred_groups: Vec::new(),
        }
    }
}

impl MangaDexOptions {
    /// The stored options; a missing or malformed entry falls back to the defaults.
    pub fn from_settings(settings: &Settings) -> Self {
        settings
            .provider_options
            .get(SOURCE_NAME)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    /// Known ratings only, in MangaDex order; an empty selection means the default.
    pub fn ratings(&self) -> Vec<&'static str> {
        let picked: Vec<&'static str> = ALL_CONTENT_RATINGS
            .into_iter()
            .filter(|r| {
                self.content_ratings
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(r))
            })
            .collect();
        if picked.is_empty() {
            vec![ALL_CONTENT_RATINGS[0], ALL_CONTENT_RATINGS[1]]
        } else {
            picked
        }
    }
}
