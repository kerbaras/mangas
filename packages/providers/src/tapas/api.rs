//! Wire types for `GET /series/{id}/episodes?page=N`, the one JSON endpoint Tapas answers
//! without an `Accept: application/json` header. Deserialisation only.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct EpisodesResponse {
    pub data: EpisodesData,
}

#[derive(Debug, Deserialize)]
pub struct EpisodesData {
    pub pagination: Pagination,
    #[serde(default)]
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    #[serde(default)]
    pub has_next: bool,
}

/// One episode as listed, oldest first. `scene` is its position in the series; early access
/// episodes report `0` until they are released and give their position in `pending_scene`.
#[derive(Debug, Clone, Deserialize)]
pub struct Episode {
    pub id: u64,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub scene: u32,
    #[serde(default)]
    pub pending_scene: u32,
    #[serde(default)]
    pub free: bool,
    #[serde(default)]
    pub early_access: bool,
    #[serde(default)]
    pub scheduled: bool,
    #[serde(default)]
    pub must_pay: bool,
}

impl Episode {
    /// Readable without an account: free, released and not behind early access.
    pub fn is_free(&self) -> bool {
        self.free && !self.early_access && !self.scheduled && !self.must_pay
    }
}
