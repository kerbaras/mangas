//! MangaDot source (<https://mangadot.net>): a JSON API behind a Cloudflare verification, so
//! requests carry the browser [`Session`] captured by the desktop app.

mod api;
mod mapping;
mod search;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, Result};
use mangas_core::sources::{Header, PageUrls, Source};

pub use mapping::{
    chapter_id, images_path, loader_status, parse_chapters, parse_images, parse_manga_list,
    parse_manga_page, site_url, SITE, SOURCE_NAME,
};
pub use search::{honoured, matches};

use crate::http::ApiClient;
use crate::session::Session;

const API_BASE: &str = "https://mangadot.net/api";
const REQUESTS_PER_SECOND: u32 = 3;
/// Titles per catalogue page when the text search has to be done locally.
const CATALOGUE_PAGE: usize = 500;
/// Upper bound of catalogue pages walked for one local search.
const CATALOGUE_MAX_PAGES: usize = 20;

pub struct MangaDot {
    api: ApiClient,
    site: ApiClient,
    session: Option<Session>,
}

impl MangaDot {
    pub fn new(session: Option<Session>) -> Result<Self> {
        Self::with_base(SITE, API_BASE, session)
    }

    /// Points the source at other hosts (tests).
    pub fn with_base(site: &str, api: &str, session: Option<Session>) -> Result<Self> {
        let rate = REQUESTS_PER_SECOND;
        Ok(Self {
            api: ApiClient::with_session(SOURCE_NAME, SITE, api, rate, session.clone())?,
            site: ApiClient::with_session(SOURCE_NAME, SITE, site, rate, session.clone())?,
            session,
        })
    }

    /// Walks the catalogue page by page, keeping titles that match `query`.
    async fn search_catalogue(&self, query: &str, limit: usize) -> Result<Vec<Manga>> {
        let mut found = Vec::new();
        for page in 1..=CATALOGUE_MAX_PAGES {
            let params = [
                ("limit", CATALOGUE_PAGE.to_string()),
                ("page", page.to_string()),
            ];
            let items = parse_manga_list(&self.api.get("/manga", &params).await?)?;
            let last = items.len() < CATALOGUE_PAGE;
            found.extend(search::filter(items, query, limit - found.len()));
            if last || found.len() >= limit {
                break;
            }
        }
        Ok(found)
    }
}

/// True when `url` is on `site` (same scheme and host, any path).
pub fn same_site(url: &str, site: &str) -> bool {
    let rest = url.strip_prefix(site).unwrap_or(url);
    rest.len() < url.len() && (rest.is_empty() || rest.starts_with('/') || rest.starts_with('?'))
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
        let params = [
            ("q", query.to_string()),
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

    /// The referer always; the browser session only to the site itself, never to a CDN or
    /// other image host a chapter may point at.
    fn page_headers(&self, url: &str) -> Vec<Header> {
        let mut headers = vec![("referer".to_string(), format!("{SITE}/"))];
        if let Some(session) = self.session.as_ref().filter(|_| same_site(url, SITE)) {
            headers.extend(session.headers());
        }
        headers
    }
}
