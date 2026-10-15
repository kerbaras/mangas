use serde::{Deserialize, Serialize};

/// Source-scoped identifier of a manga (e.g. the MangaDex UUID).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MangaId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MangaStatus {
    Ongoing,
    Completed,
    Hiatus,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manga {
    pub id: MangaId,
    pub source: String,
    pub title: String,
    pub description: String,
    pub cover_url: Option<String>,
    pub status: MangaStatus,
}

impl std::fmt::Display for MangaId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
