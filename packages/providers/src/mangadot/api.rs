//! Wire types for the subset of the MangaDot API we read (`/api/...`). Deserialisation only;
//! mapping to domain types lives in `mapping`.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MangaList {
    #[serde(default)]
    pub manga_list: Vec<MangaEntry>,
}

#[derive(Debug, Deserialize)]
pub struct MangaEntry {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub photo: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChapterEntry {
    pub id: u64,
    #[serde(default)]
    pub chapter_number: Option<serde_json::Value>,
    #[serde(default)]
    pub volume_number: Option<serde_json::Value>,
    #[serde(default)]
    pub chapter_title: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    /// "scraper" for mirrored chapters, "user" for uploads (served from another endpoint).
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChapterImages {
    #[serde(default)]
    pub images: Vec<Image>,
}

#[derive(Debug, Deserialize)]
pub struct Image {
    pub url: String,
}
