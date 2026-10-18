//! Wire types for the Webtoons JSON we read: the search autocomplete and the mobile episode
//! list. Deserialisation only; mapping lives in `mapping`.

use serde::Deserialize;

/// `GET /{lang}/search/immediate?keyword=` (originals only).
#[derive(Debug, Deserialize)]
pub struct SearchReply {
    pub result: Option<SearchResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    #[serde(default)]
    pub searched_list: Vec<SearchEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchEntry {
    /// Null on `searchMode: "AUTHOR"` entries, which list an author's titles by name only.
    #[serde(default)]
    pub title_no: Option<u64>,
    #[serde(default)]
    pub title: Option<String>,
    /// Path on the image CDN (`/20230324_171/x.jpg`).
    #[serde(default)]
    pub thumbnail_mobile: Option<String>,
    #[serde(default)]
    pub author_name_list: Vec<String>,
}

/// `GET m.webtoons.com/api/v1/{webtoon|canvas}/{titleNo}/episodes`; `result` is null for an
/// unknown title.
#[derive(Debug, Deserialize)]
pub struct EpisodesReply {
    pub result: Option<EpisodePage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodePage {
    #[serde(default)]
    pub episode_list: Vec<EpisodeEntry>,
    /// Cursor of the next page; `0` (or absent) on the last one.
    #[serde(default)]
    pub next_cursor: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeEntry {
    pub episode_no: u64,
    #[serde(default)]
    pub episode_title: Option<String>,
}
