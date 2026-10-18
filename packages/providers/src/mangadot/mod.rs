//! MangaDot source (<https://mangadot.net>): a JSON API behind a Cloudflare rule that only lets
//! same-origin-looking requests (the site `Referer`) through. No browser [`Session`] is needed;
//! a stored one is still sent, and a challenge still asks for one (#94).

mod api;
mod mapping;
mod metadata;

use std::sync::Arc;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, Result};
use mangas_core::library::Settings;
use mangas_core::sources::{Header, PageUrls, ProviderStatus, Source};

pub use mapping::{
    chapter_id, images_path, parse_chapters, parse_images, parse_manga_detail, parse_manga_list,
    site_url, SITE, SOURCE_NAME,
};

use crate::http::{ClientConfig, SiteClient};
use crate::provider::{Capability, Provider, ProviderInfo};
use crate::session::Session;

const API_BASE: &str = "https://mangadot.net/api";
const REQUESTS_PER_SECOND: u32 = 3;
/// One title of the catalogue: the cheapest API call that passes through the verification.
const PROBE: &str = "/manga?limit=1&page=1";

pub const INFO: ProviderInfo = ProviderInfo {
    id: SOURCE_NAME,
    name: "MangaDot",
    site: SITE,
    languages: &["en"],
    capabilities: &[
        Capability::Search,
        Capability::Chapters,
        Capability::Download,
    ],
    needs_browser_session: false,
    icon_url: Some("https://mangadot.net/favicon.ico"),
};

pub struct MangaDotProvider;

impl Provider for MangaDotProvider {
    fn info(&self) -> ProviderInfo {
        INFO
    }

    fn build(&self, settings: &Settings) -> Result<Arc<dyn Source>> {
        self.build_at(settings, SITE)
    }

    fn build_at(&self, settings: &Settings, base: &str) -> Result<Arc<dyn Source>> {
        let api = format!("{}/api", base.trim_end_matches('/'));
        let session = Session::from_settings(settings, INFO.id);
        Ok(Arc::new(MangaDot::with_base(&api, session)?))
    }
}

/// Everything goes through the API: the site's HTML pages stay challenged for any client that is
/// not the browser, even with a valid session (#94).
pub struct MangaDot {
    api: SiteClient,
}

impl MangaDot {
    pub fn new(session: Option<Session>) -> Result<Self> {
        Self::with_base(API_BASE, session)
    }

    /// Points the source at another API host (tests).
    pub fn with_base(api: &str, session: Option<Session>) -> Result<Self> {
        let config = ClientConfig {
            requests_per_second: REQUESTS_PER_SECOND,
            // Mirrored chapters point at other sites' CDNs; they all get the referer.
            image_hosts: &["*"],
            ..ClientConfig::new(INFO, api)
        };
        Ok(Self {
            api: SiteClient::new(config, session)?,
        })
    }
}

#[async_trait]
impl Source for MangaDot {
    fn name(&self) -> &'static str {
        SOURCE_NAME
    }

    /// The API ranks by relevance and pads the tail with fuzzy matches; its order is kept.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Manga>> {
        let limit = limit.clamp(1, 100);
        let params = [
            ("search", query.to_string()),
            ("limit", limit.to_string()),
            ("page", "1".to_string()),
        ];
        let items = parse_manga_list(&self.api.get("/search", &params).await?)?;
        Ok(items.into_iter().take(limit).collect())
    }

    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        let body = self.api.get(&format!("/manga/{id}"), &[]).await?;
        parse_manga_detail(id, &body)
    }

    async fn chapters(&self, id: &MangaId, language: &str) -> Result<Vec<Chapter>> {
        let body = self
            .api
            .get(&format!("/manga/{id}/chapters/list"), &[])
            .await?;
        parse_chapters(&body, id, language)
    }

    async fn pages(&self, chapter: &ChapterId) -> Result<PageUrls> {
        let body = self.api.get(&images_path(chapter)?, &[]).await?;
        parse_images(&body)
    }

    fn page_headers(&self, url: &str) -> Vec<Header> {
        self.api.page_headers(url)
    }

    async fn status(&self) -> ProviderStatus {
        self.api.status(PROBE).await
    }
}
