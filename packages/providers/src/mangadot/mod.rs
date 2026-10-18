//! MangaDot source (<https://mangadot.net>): a JSON API behind a Cloudflare verification, so
//! requests carry the browser [`Session`] captured by the desktop app.

mod api;
mod mapping;
mod search;

use std::sync::Arc;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, Result};
use mangas_core::library::Settings;
use mangas_core::sources::{Header, PageUrls, ProviderStatus, Source};

pub use mapping::{
    chapter_id, images_path, loader_status, parse_chapters, parse_images, parse_manga_list,
    parse_manga_page, site_url, SITE, SOURCE_NAME,
};
pub use search::{honoured, matches};

use crate::http::{ClientConfig, SiteClient};
use crate::provider::{Capability, Provider, ProviderInfo};
use crate::session::Session;

const API_BASE: &str = "https://mangadot.net/api";
const REQUESTS_PER_SECOND: u32 = 3;
/// One title of the catalogue: the cheapest API call that passes through the verification.
const PROBE: &str = "/manga?limit=1&page=1";
/// Titles per catalogue page when the text search has to be done locally.
const CATALOGUE_PAGE: usize = 500;
/// Upper bound of catalogue pages walked for one local search.
const CATALOGUE_MAX_PAGES: usize = 20;

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
    needs_browser_session: true,
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
        let base = base.trim_end_matches('/');
        let session = Session::from_settings(settings, INFO.id);
        Ok(Arc::new(MangaDot::with_base(
            base,
            &format!("{base}/api"),
            session,
        )?))
    }
}

pub struct MangaDot {
    api: SiteClient,
    site: SiteClient,
}

impl MangaDot {
    pub fn new(session: Option<Session>) -> Result<Self> {
        Self::with_base(SITE, API_BASE, session)
    }

    /// Points the source at other hosts (tests).
    pub fn with_base(site: &str, api: &str, session: Option<Session>) -> Result<Self> {
        let config = |base| ClientConfig {
            requests_per_second: REQUESTS_PER_SECOND,
            // Mirrored chapters point at other sites' CDNs; they all get the referer.
            image_hosts: &["*"],
            ..ClientConfig::new(INFO, base)
        };
        Ok(Self {
            api: SiteClient::new(config(api), session.clone())?,
            site: SiteClient::new(config(site), session)?,
        })
    }

    /// Walks the catalogue page by page, keeping titles that match `query`, until an empty page,
    /// `limit` matches or [`CATALOGUE_MAX_PAGES`].
    async fn search_catalogue(&self, query: &str, limit: usize) -> Result<Vec<Manga>> {
        let mut found = Vec::new();
        for page in 1..=CATALOGUE_MAX_PAGES {
            let params = [
                ("limit", CATALOGUE_PAGE.to_string()),
                ("page", page.to_string()),
            ];
            let items = parse_manga_list(&self.api.get("/manga", &params).await?)?;
            // Like the site's own lister: only an empty page ends the catalogue.
            if items.is_empty() {
                break;
            }
            found.extend(search::filter(items, query, limit - found.len()));
            if found.len() >= limit {
                break;
            }
        }
        Ok(found)
    }
}

#[async_trait]
impl Source for MangaDot {
    fn name(&self) -> &'static str {
        SOURCE_NAME
    }

    /// Tries the search endpoint first; if it ignores the text (the results do not mostly
    /// match it), filters the catalogue locally instead.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Manga>> {
        let limit = limit.clamp(1, 100);
        // `search` is the name the site's own search page uses for the text (its route reads
        // and writes `?search=`); the API is assumed to take the same one.
        let params = [
            ("search", query.to_string()),
            ("limit", limit.to_string()),
            ("page", "1".to_string()),
        ];
        let items = parse_manga_list(&self.api.get("/search", &params).await?)?;
        if search::honoured(&items, query) {
            Ok(search::filter(items, query, limit))
        } else {
            self.search_catalogue(query, limit).await
        }
    }

    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        let html = self.site.get(&format!("/manga/{id}"), &[]).await?;
        parse_manga_page(id, &html)
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
