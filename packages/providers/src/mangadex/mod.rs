//! MangaDex source (<https://api.mangadex.org>).

mod api;
mod mapping;

use std::sync::Arc;

use async_trait::async_trait;
use mangas_core::library::Settings;

use crate::http::{ClientConfig, SiteClient};
use crate::provider::{Capability, Provider, ProviderInfo};

pub const API_BASE: &str = "https://api.mangadex.org";
/// MangaDex's global limit is 5 requests per second per client.
const REQUESTS_PER_SECOND: u32 = 5;
/// Cheap health endpoint (`pong`).
const PROBE: &str = "/ping";
pub use mapping::{
    cover_url, dedupe_and_sort, parse_at_home_response, parse_error_response, parse_feed_response,
    parse_manga_response, parse_search_response, FeedPage, SOURCE_NAME,
};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, Result};
use mangas_core::sources::{Header, PageUrls, ProviderStatus, Source};

const FEED_PAGE_SIZE: u32 = 500;

pub const INFO: ProviderInfo = ProviderInfo {
    id: SOURCE_NAME,
    name: "MangaDex",
    site: "https://mangadex.org",
    languages: &[
        "en", "ja", "ko", "zh", "zh-hk", "es", "es-la", "fr", "de", "it", "pt-br", "ru", "pl",
        "tr", "id", "vi", "th",
    ],
    capabilities: &[
        Capability::Search,
        Capability::Chapters,
        Capability::Download,
    ],
    needs_browser_session: false,
    icon_url: Some("https://mangadex.org/favicon.ico"),
};

pub struct MangaDexProvider;

impl Provider for MangaDexProvider {
    fn info(&self) -> ProviderInfo {
        INFO
    }

    fn build(&self, _settings: &Settings) -> Result<Arc<dyn Source>> {
        Ok(Arc::new(MangaDex::new()?))
    }

    fn build_at(&self, _settings: &Settings, base: &str) -> Result<Arc<dyn Source>> {
        Ok(Arc::new(MangaDex::with_base(base)?))
    }
}

pub struct MangaDex {
    client: SiteClient,
}

impl MangaDex {
    pub fn new() -> Result<Self> {
        Self::with_base(API_BASE)
    }

    /// Points the source at another base URL (tests, mirrors).
    pub fn with_base(base: &str) -> Result<Self> {
        let config = ClientConfig {
            requests_per_second: REQUESTS_PER_SECOND,
            ..ClientConfig::new(INFO, base)
        };
        Ok(Self {
            client: SiteClient::new(config, None)?,
        })
    }
}

impl MangaDex {
    /// GET returning the body; MangaDex error payloads become [`mangas_core::Error::Source`].
    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<String> {
        let reply = self.client.get_raw(path, query).await?;
        if (200..300).contains(&reply.status) {
            Ok(reply.body)
        } else {
            Err(parse_error_response(reply.status, &reply.body))
        }
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
        let body = self.get("/manga", &params).await?;
        parse_search_response(&body)
    }

    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        let params = [("includes[]", "cover_art".to_string())];
        let body = self.get(&format!("/manga/{id}"), &params).await?;
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
            let body = self.get(&path, &params).await?;
            let page = parse_feed_response(&body, id, language)?;
            all.extend(page.chapters);
            offset = page.next_offset;
        }
        Ok(dedupe_and_sort(all))
    }

    async fn pages(&self, chapter: &ChapterId) -> Result<PageUrls> {
        let body = self.get(&format!("/at-home/server/{chapter}"), &[]).await?;
        parse_at_home_response(&body)
    }

    fn page_headers(&self, url: &str) -> Vec<Header> {
        self.client.page_headers(url)
    }

    async fn status(&self) -> ProviderStatus {
        self.client.status(PROBE).await
    }
}
