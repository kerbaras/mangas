//! Comick source (<https://comick.live>, the Comick that still hosts chapters since the
//! original comick.io/.dev stopped in 2025). Comic pages, chapter lists and reader pages are
//! open; search sits behind a Cloudflare verification, so it uses the browser [`Session`]
//! captured by the desktop app.

mod api;
mod mapping;
mod page_data;

use std::sync::Arc;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, Result};
use mangas_core::library::Settings;
use mangas_core::sources::{Header, PageUrls, ProviderStatus, Source};

pub use mapping::{
    chapter_id, chapter_path, parse_chapter_page, parse_comic_page, parse_reader_page,
    parse_search, parse_status, SITE, SOURCE_NAME,
};
pub use page_data::{plain_text, script_json};

use crate::http::{ClientConfig, SiteClient};
use crate::provider::{Capability, Provider, ProviderInfo};
use crate::session::Session;

/// keiyoushi's limit for this site; its image CDN answers 429 to bursts.
const REQUESTS_PER_SECOND: u32 = 2;
/// Search is the gated endpoint, so it is what tells whether the session still works.
const PROBE: &str = "/api/search?q=solo&type=comic";
/// Upper bound of chapter-list pages (60 chapters each) walked for one title.
const MAX_CHAPTER_PAGES: u32 = 200;

pub const INFO: ProviderInfo = ProviderInfo {
    id: SOURCE_NAME,
    name: "Comick",
    site: SITE,
    languages: &[
        "en", "es-419", "pt-br", "fr", "it", "de", "ru", "id", "vi", "pl", "tr", "uk", "th",
    ],
    capabilities: &[
        Capability::Search,
        Capability::Chapters,
        Capability::Download,
    ],
    needs_browser_session: true,
    icon_url: Some("https://comick.live/favicon.ico"),
};

pub struct ComickProvider;

impl Provider for ComickProvider {
    fn info(&self) -> ProviderInfo {
        INFO
    }

    fn build(&self, settings: &Settings) -> Result<Arc<dyn Source>> {
        self.build_at(settings, SITE)
    }

    fn build_at(&self, settings: &Settings, base: &str) -> Result<Arc<dyn Source>> {
        let session = Session::from_settings(settings, INFO.id);
        Ok(Arc::new(Comick::with_base(base, session)?))
    }
}

pub struct Comick {
    client: SiteClient,
}

impl Comick {
    pub fn new(session: Option<Session>) -> Result<Self> {
        Self::with_base(SITE, session)
    }

    /// Points the source at another host (tests).
    pub fn with_base(base: &str, session: Option<Session>) -> Result<Self> {
        let config = ClientConfig {
            requests_per_second: REQUESTS_PER_SECOND,
            // Pages and covers: 403 without the site's referer.
            image_hosts: &["comicknew.pictures"],
            ..ClientConfig::new(INFO, base)
        };
        Ok(Self {
            client: SiteClient::new(config, session)?,
        })
    }
}

#[async_trait]
impl Source for Comick {
    fn name(&self) -> &'static str {
        SOURCE_NAME
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Manga>> {
        let params = [("q", query.trim().to_string()), ("type", "comic".into())];
        let body = self.client.get("/api/search", &params).await?;
        Ok(parse_search(&body)?
            .into_iter()
            .take(limit.max(1))
            .collect())
    }

    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        let html = self.client.get(&format!("/comic/{id}"), &[]).await?;
        parse_comic_page(id, &html)
    }

    async fn chapters(&self, id: &MangaId, language: &str) -> Result<Vec<Chapter>> {
        let path = format!("/api/comics/{id}/chapter-list");
        let mut pages = Vec::new();
        let mut page = 1;
        loop {
            let params = [("lang", language.to_string()), ("page", page.to_string())];
            let body = self.client.get(&path, &params).await?;
            let (chapters, last) = parse_chapter_page(&body, id, language)?;
            pages.push(chapters);
            if page >= last.min(MAX_CHAPTER_PAGES) {
                break;
            }
            page += 1;
        }
        Ok(mapping::sorted(pages))
    }

    async fn pages(&self, chapter: &ChapterId) -> Result<PageUrls> {
        let html = self.client.get(&chapter_path(chapter)?, &[]).await?;
        parse_reader_page(chapter, &html)
    }

    fn page_headers(&self, url: &str) -> Vec<Header> {
        self.client.page_headers(url)
    }

    async fn status(&self) -> ProviderStatus {
        self.client.status(PROBE).await
    }
}
