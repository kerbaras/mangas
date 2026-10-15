//! MangaDex source (<https://api.mangadex.org>).

mod api;
mod client;
mod mapping;
mod rate_limit;

use async_trait::async_trait;

pub use client::API_BASE;
pub use mapping::{
    cover_url, dedupe_and_sort, parse_at_home_response, parse_error_response, parse_feed_response,
    parse_manga_response, parse_search_response, FeedPage, SOURCE_NAME,
};

use crate::domain::{Chapter, ChapterId, Manga, MangaId, Result};
use crate::sources::{PageUrls, Source};

const FEED_PAGE_SIZE: u32 = 500;

pub struct MangaDex {
    client: client::ApiClient,
}

impl MangaDex {
    pub fn new() -> Result<Self> {
        Self::with_base(API_BASE)
    }

    /// Points the source at another base URL (tests, mirrors).
    pub fn with_base(base: &str) -> Result<Self> {
        Ok(Self {
            client: client::ApiClient::new(base)?,
        })
    }
}

#[async_trait]
impl Source for MangaDex {
    fn name(&self) -> &'static str {
        SOURCE_NAME
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Manga>> {
        let params = [
            ("title", query.to_string()),
            ("limit", limit.clamp(1, 100).to_string()),
            ("includes[]", "cover_art".to_string()),
        ];
        let body = self.client.get("/manga", &params).await?;
        parse_search_response(&body)
    }

    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        let params = [("includes[]", "cover_art".to_string())];
        let body = self.client.get(&format!("/manga/{id}"), &params).await?;
        parse_manga_response(&body)
    }

    async fn chapters(&self, id: &MangaId, language: &str) -> Result<Vec<Chapter>> {
        let path = format!("/manga/{id}/feed");
        let mut all = Vec::new();
        let mut offset = Some(0);
        while let Some(current) = offset {
            let params = [
                ("translatedLanguage[]", language.to_string()),
                ("order[chapter]", "asc".to_string()),
                ("limit", FEED_PAGE_SIZE.to_string()),
                ("offset", current.to_string()),
            ];
            let body = self.client.get(&path, &params).await?;
            let page = parse_feed_response(&body, id, language)?;
            all.extend(page.chapters);
            offset = page.next_offset;
        }
        Ok(dedupe_and_sort(all))
    }

    async fn pages(&self, chapter: &ChapterId) -> Result<PageUrls> {
        let body = self
            .client
            .get(&format!("/at-home/server/{chapter}"), &[])
            .await?;
        parse_at_home_response(&body)
    }
}
