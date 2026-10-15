use serde::{Deserialize, Serialize};

use super::MangaId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChapterId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chapter {
    pub id: ChapterId,
    pub manga_id: MangaId,
    pub title: Option<String>,
    pub language: String,
    pub volume: Option<String>,
    /// Chapter number as published ("12", "12.5"); kept as text to preserve the source's labelling.
    pub number: String,
    /// Local directory with the downloaded pages, once downloaded.
    pub local_path: Option<String>,
}

impl Chapter {
    pub fn is_downloaded(&self) -> bool {
        self.local_path.is_some()
    }
}

impl std::fmt::Display for ChapterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
