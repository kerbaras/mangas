//! MangaDex source (<https://api.mangadex.org>).

mod api;
mod dedupe;
mod external;
mod mapping;
mod options;
mod pages;
mod query;

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
pub use dedupe::{dedupe_chapters, FeedChapter};
pub use external::secure_link;
pub use mapping::{
    cover_url, error_detail, parse_error_response, parse_feed_response, parse_manga_response,
    parse_search_response, FeedPage, SOURCE_NAME,
};
pub use options::{MangaDexOptions, ALL_CONTENT_RATINGS};
pub use pages::{parse_at_home_response, uploads_mirror, UPLOADS_ORIGIN};
pub use query::{feed_query, manga_query, search_query, FEED_PAGE_SIZE};

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, Result};
use mangas_core::sources::{Header, PageUrls, ProviderStatus, Source};

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

    fn build(&self, settings: &Settings) -> Result<Arc<dyn Source>> {
        let source = MangaDex::new()?.with_preferences(
            MangaDexOptions::from_settings(settings),
            settings.languages_for(INFO.id),
        );
        Ok(Arc::new(source))
    }

    fn build_at(&self, _settings: &Settings, base: &str) -> Result<Arc<dyn Source>> {
        Ok(Arc::new(MangaDex::with_base(base)?))
    }

    fn options(&self, settings: &Settings) -> Option<serde_json::Value> {
        let stored = MangaDexOptions::from_settings(settings);
        let effective = MangaDexOptions {
            content_ratings: stored.ratings().into_iter().map(String::from).collect(),
            ..stored
        };
        serde_json::to_value(effective).ok()
    }
}

pub struct MangaDex {
    client: SiteClient,
    options: MangaDexOptions,
    languages: Vec<String>,
}

impl MangaDex {
    pub fn new() -> Result<Self> {
        Self::with_base(API_BASE)
    }

    /// Points the source at another base URL (tests, mirrors).
    pub fn with_base(base: &str) -> Result<Self> {
        let config = ClientConfig {
            requests_per_second: REQUESTS_PER_SECOND,
            json_api: true,
            ..ClientConfig::new(INFO, base)
        };
        Ok(Self {
            client: SiteClient::new(config, None)?,
            options: MangaDexOptions::default(),
            languages: Settings::default().languages(),
        })
    }

    /// Search filters, data-saver and group preference from the user's settings.
    pub fn with_preferences(self, options: MangaDexOptions, languages: Vec<String>) -> Self {
        Self {
            options,
            languages,
            ..self
        }
    }
}

impl MangaDex {
    /// GET returning the body; a failure keeps the MangaDex error `detail` ([`error_detail`]).
    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<String> {
        self.client.get_explained(path, query, error_detail).await
    }
}

#[async_trait]
impl Source for MangaDex {
    fn name(&self) -> &'static str {
        SOURCE_NAME
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Manga>> {
        let ratings = self.options.ratings();
        let params = search_query(query, limit, &ratings, &self.languages);
        let body = self.get("/manga", &params).await?;
        parse_search_response(&body)
    }

    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        let body = self.get(&format!("/manga/{id}"), &manga_query()).await?;
        parse_manga_response(&body)
    }

    async fn chapters(&self, id: &MangaId, language: &str) -> Result<Vec<Chapter>> {
        let path = format!("/manga/{id}/feed");
        let mut all = Vec::new();
        let mut offset = Some(0);
        while let Some(current) = offset {
            let body = self.get(&path, &feed_query(language, current)).await?;
            let page = parse_feed_response(&body, id, language)?;
            all.extend(page.chapters);
            offset = page.next_offset;
        }
        Ok(dedupe_chapters(all, &self.options.preferred_groups))
    }

    async fn pages(&self, chapter: &ChapterId) -> Result<PageUrls> {
        let body = self.get(&format!("/at-home/server/{chapter}"), &[]).await?;
        parse_at_home_response(&body, self.options.data_saver)
    }

    fn page_headers(&self, url: &str) -> Vec<Header> {
        self.client.page_headers(url)
    }

    fn page_url(&self, url: &str) -> String {
        self.client.page_url(url)
    }

    fn mirror_url(&self, url: &str) -> Option<String> {
        uploads_mirror(url)
    }

    async fn status(&self) -> ProviderStatus {
        self.client.status(PROBE).await
    }
}
