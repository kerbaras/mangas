//! Wire types for the parts of comick.live we read. Deserialisation only; mapping to domain
//! types lives in `mapping`.

use serde::Deserialize;
use serde_json::Value;

/// `/api/search` (and the same item shape from `/api/comics/top`).
#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    #[serde(default)]
    pub data: Vec<ComicItem>,
}

#[derive(Debug, Deserialize)]
pub struct ComicItem {
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub default_thumbnail: Option<String>,
}

/// The `#comic-data` JSON inlined in `/comic/{slug}`.
#[derive(Debug, Deserialize)]
pub struct ComicData {
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub default_thumbnail: Option<String>,
    #[serde(default)]
    pub status: Option<i64>,
}

/// `/api/comics/{slug}/chapter-list`: one page of chapters, newest first.
#[derive(Debug, Deserialize)]
pub struct ChapterList {
    #[serde(default)]
    pub data: Vec<ChapterEntry>,
    pub pagination: Pagination,
}

#[derive(Debug, Deserialize)]
pub struct ChapterEntry {
    pub hid: String,
    #[serde(default)]
    pub chap: Option<Value>,
    #[serde(default)]
    pub vol: Option<Value>,
    #[serde(default)]
    pub title: Option<String>,
    pub lang: String,
    #[serde(default)]
    pub group_name: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub last_page: u32,
}

/// The `#sv-data` JSON inlined in a chapter page.
#[derive(Debug, Deserialize)]
pub struct ReaderData {
    pub chapter: ReaderChapter,
}

#[derive(Debug, Deserialize)]
pub struct ReaderChapter {
    #[serde(default)]
    pub images: Vec<Image>,
}

#[derive(Debug, Deserialize)]
pub struct Image {
    pub url: String,
}
