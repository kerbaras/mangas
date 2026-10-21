//! Wire types for the subset of the MangaDex API we read. Only deserialisation; mapping to
//! domain types lives in `mapping`.

use std::collections::BTreeMap;

use serde::Deserialize;

/// Text localised by language code ("en", "ja-ro", ...).
pub type Localized = BTreeMap<String, String>;

#[derive(Debug, Deserialize)]
pub struct Collection<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub total: u32,
}

#[derive(Debug, Deserialize)]
pub struct Entity<T> {
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct MangaData {
    pub id: String,
    pub attributes: MangaAttributes,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Deserialize)]
pub struct MangaAttributes {
    #[serde(default)]
    pub title: Localized,
    #[serde(default, rename = "altTitles")]
    pub alt_titles: Vec<Localized>,
    #[serde(default)]
    pub description: Localized,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub year: Option<u16>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default, rename = "originalLanguage")]
    pub original_language: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Tag {
    pub attributes: TagAttributes,
}

#[derive(Debug, Deserialize)]
pub struct TagAttributes {
    #[serde(default)]
    pub name: Localized,
}

#[derive(Debug, Deserialize)]
pub struct Relationship {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub attributes: Option<RelationshipAttributes>,
}

#[derive(Debug, Deserialize)]
pub struct RelationshipAttributes {
    /// `cover_art` file.
    #[serde(default, rename = "fileName")]
    pub file_name: Option<String>,
    /// `scanlation_group`, `author` and `artist` name.
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChapterData {
    pub id: String,
    pub attributes: ChapterAttributes,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Deserialize)]
pub struct ChapterAttributes {
    #[serde(default)]
    pub volume: Option<String>,
    #[serde(default)]
    pub chapter: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(rename = "translatedLanguage")]
    pub translated_language: String,
    #[serde(default, rename = "externalUrl")]
    pub external_url: Option<String>,
    #[serde(default, rename = "isUnavailable")]
    pub is_unavailable: bool,
    /// Page count; 0 for chapters hosted elsewhere.
    #[serde(default)]
    pub pages: Option<u32>,
    /// RFC 3339 in UTC (`2024-01-31T12:00:00+00:00`), so it compares as text.
    #[serde(default, rename = "publishAt")]
    pub publish_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AtHome {
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    pub chapter: AtHomeChapter,
}

#[derive(Debug, Deserialize)]
pub struct AtHomeChapter {
    pub hash: String,
    #[serde(default)]
    pub data: Vec<String>,
    #[serde(default, rename = "dataSaver")]
    pub data_saver: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ErrorBody {
    #[serde(default)]
    pub errors: Vec<ApiError>,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub detail: Option<String>,
}
