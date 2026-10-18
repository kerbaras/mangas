//! Webtoons (LINE) source (<https://www.webtoons.com>): originals and canvas titles in every
//! language edition Webtoons publishes. Search and episodes are JSON; the title page and the
//! viewer are HTML. Page images need the site's referer (`webtoon-phinf` answers 403 without).

mod api;
mod html;
mod ids;
mod mapping;

use std::sync::Arc;

use async_trait::async_trait;
use futures::future::try_join_all;
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, Result};
use mangas_core::library::Settings;
use mangas_core::sources::{Header, PageUrls, ProviderStatus, Source};

pub use html::{canvas_cards, full_size, unescape, viewer_images, Card};
pub use ids::{Episode, Title};
pub use mapping::{
    cover_url, parse_canvas_search, parse_episode_page, parse_search, parse_title_page,
    parse_viewer, supported, title_status, to_chapters,
};

use crate::http::{ClientConfig, SiteClient};
use crate::provider::{Capability, Provider, ProviderInfo};
use crate::session::Session;

pub const SOURCE_NAME: &str = "webtoons";
pub const SITE: &str = "https://www.webtoons.com";
const MOBILE: &str = "https://m.webtoons.com";
/// Episodes per request; the API pages with `nextCursor`.
const PAGE_SIZE: usize = 1000;
/// Upper bound of episode pages walked for one title.
const MAX_PAGES: usize = 20;
/// Most originals the search autocomplete returns per request (its default is ten).
const MAX_DISPLAY: usize = 100;
const PROBE: &str = "/en/search/immediate?keyword=webtoon";

pub const INFO: ProviderInfo = ProviderInfo {
    id: SOURCE_NAME,
    name: "Webtoons",
    site: SITE,
    languages: &["en", "es", "fr", "de", "id", "th", "zh-hant"],
    capabilities: &[
        Capability::Search,
        Capability::Chapters,
        Capability::Download,
    ],
    needs_browser_session: false,
    icon_url: Some("https://webtoons-static.pstatic.net/image/favicon/favicon.ico"),
};

pub struct WebtoonsProvider;

impl Provider for WebtoonsProvider {
    fn info(&self) -> ProviderInfo {
        INFO
    }

    fn build(&self, settings: &Settings) -> Result<Arc<dyn Source>> {
        Ok(Arc::new(Webtoons::with_bases(SITE, MOBILE, settings)?))
    }

    /// Both hosts go to `base`: their paths never collide (`/api/v1/…` is only on `m.`).
    fn build_at(&self, settings: &Settings, base: &str) -> Result<Arc<dyn Source>> {
        Ok(Arc::new(Webtoons::with_bases(base, base, settings)?))
    }
}

pub struct Webtoons {
    site: SiteClient,
    mobile: SiteClient,
    /// Preferred languages Webtoons publishes in, searched in this order.
    languages: Vec<String>,
}

impl Webtoons {
    pub fn with_bases(site: &str, mobile: &str, settings: &Settings) -> Result<Self> {
        let config = |base| ClientConfig {
            image_hosts: &["webtoon-phinf.pstatic.net", "swebtoon-phinf.pstatic.net"],
            ..ClientConfig::new(INFO, base)
        };
        let session = Session::from_settings(settings, INFO.id);
        Ok(Self {
            site: SiteClient::new(config(site), session)?,
            // The API needs no session; host-only `www` cookies must not reach `m.`.
            mobile: SiteClient::new(config(mobile), None)?,
            languages: supported(&settings.languages_for(INFO.id)),
        })
    }

    /// Originals (JSON) and canvas titles (HTML) of one language edition.
    async fn search_language(
        &self,
        query: &str,
        language: &str,
        limit: usize,
    ) -> Result<Vec<Manga>> {
        let params = [("keyword", query.to_string())];
        let display = ("display", limit.clamp(1, MAX_DISPLAY).to_string());
        let originals_params = [params[0].clone(), display];
        let (originals_path, canvas_path) = (
            format!("/{language}/search/immediate"),
            format!("/{language}/search/canvas"),
        );
        let originals = self.site.get(&originals_path, &originals_params);
        let canvas = self.site.get_raw(&canvas_path, &params);
        let (originals, canvas) = futures::try_join!(originals, canvas)?;
        let mut found = parse_search(&originals, language)?;
        match canvas.status {
            200..=299 => found.extend(parse_canvas_search(&canvas.body, language)),
            404 => {}
            status => log::warn!("{SOURCE_NAME}: canvas search HTTP {status}"),
        }
        Ok(found)
    }
}

#[async_trait]
impl Source for Webtoons {
    fn name(&self) -> &'static str {
        SOURCE_NAME
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Manga>> {
        let editions = self
            .languages
            .iter()
            .map(|l| self.search_language(query, l, limit));
        let mut found: Vec<Manga> = Vec::new();
        for manga in try_join_all(editions).await?.into_iter().flatten() {
            if !found.iter().any(|m| m.id == manga.id) {
                found.push(manga);
            }
        }
        found.truncate(limit.max(1));
        Ok(found)
    }

    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        let title = Title::parse(id)?;
        let html = self
            .site
            .get(title.list_path(), &title.list_query())
            .await?;
        parse_title_page(id, &html)
    }

    /// A title exists in one language only; asking for another is an empty list.
    async fn chapters(&self, id: &MangaId, language: &str) -> Result<Vec<Chapter>> {
        let title = Title::parse(id)?;
        if title.language != language {
            return Ok(Vec::new());
        }
        let (path, mut pages, mut cursor) = (title.episodes_path(), Vec::new(), None);
        for _ in 0..MAX_PAGES {
            let query = title.episodes_query(PAGE_SIZE, cursor);
            let page = parse_episode_page(&self.mobile.get(&path, &query).await?, &title)?;
            let next = page
                .next_cursor
                .filter(|&c| c > 0 && page.episode_list.len() >= PAGE_SIZE);
            pages.push(page);
            match next {
                Some(next) if Some(next) != cursor => cursor = Some(next),
                _ => break,
            }
        }
        Ok(to_chapters(id, &title, pages))
    }

    async fn pages(&self, chapter: &ChapterId) -> Result<PageUrls> {
        let episode = Episode::parse(chapter)?;
        let html = self
            .site
            .get(&episode.viewer_path(), &episode.viewer_query())
            .await?;
        parse_viewer(&html)
    }

    fn page_headers(&self, url: &str) -> Vec<Header> {
        self.site.page_headers(url)
    }

    fn page_url(&self, url: &str) -> String {
        self.site.page_url(url)
    }

    async fn status(&self) -> ProviderStatus {
        self.site.status(PROBE).await
    }
}
