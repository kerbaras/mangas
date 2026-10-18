//! Tapas source (<https://tapas.io>): search, series and episode pages are server-rendered
//! HTML, the episode list is JSON. Free episodes download; locked ones (paid, wait-until-free,
//! early access) are listed as [`mangas_core::domain::Availability::Locked`].

mod api;
mod html;
mod mapping;

use std::sync::Arc;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, Result};
use mangas_core::library::Settings;
use mangas_core::sources::{Header, PageUrls, ProviderStatus, Source};

pub use api::Episode;
pub use mapping::{
    next_episode_page, parse_episodes, parse_pages, parse_search, parse_series, to_chapters, SITE,
    SOURCE_NAME,
};

use crate::http::{ClientConfig, SiteClient};
use crate::provider::{Capability, Provider, ProviderInfo};
use crate::session::Session;

/// Mature series and episodes redirect to an age check without these cookies (HaruNeko sets
/// the same ones). They are the only "session" Tapas gets; no account is ever used.
const AGE_COOKIES: &str = "birthDate=2000-01-01; adjustedBirthDate=2000-01-01";
/// Episode list of a series running since 2013: a small JSON answer from the same backend.
const PROBE: &str = "/series/2101/episodes?page=1";
/// Upper bound of episode-list pages (20 episodes each) walked for one series; a longer list
/// is an error rather than a silently truncated chapter list.
pub const MAX_EPISODE_PAGES: u32 = 500;

pub const INFO: ProviderInfo = ProviderInfo {
    id: SOURCE_NAME,
    name: "Tapas",
    site: SITE,
    languages: &["en"],
    capabilities: &[
        Capability::Search,
        Capability::Chapters,
        Capability::Download,
    ],
    needs_browser_session: false,
    icon_url: Some("https://tapas.io/favicon.ico"),
};

pub struct TapasProvider;

impl Provider for TapasProvider {
    fn info(&self) -> ProviderInfo {
        INFO
    }

    fn build(&self, settings: &Settings) -> Result<Arc<dyn Source>> {
        self.build_at(settings, SITE)
    }

    fn build_at(&self, _settings: &Settings, base: &str) -> Result<Arc<dyn Source>> {
        Ok(Arc::new(Tapas::with_base(base)?))
    }
}

pub struct Tapas {
    client: SiteClient,
}

impl Tapas {
    pub fn new() -> Result<Self> {
        Self::with_base(SITE)
    }

    /// Points the source at another host (tests).
    pub fn with_base(base: &str) -> Result<Self> {
        let age = Session {
            cookies: AGE_COOKIES.to_string(),
            user_agent: String::new(),
            saved_at: None,
        };
        let config = ClientConfig::new(INFO, base);
        Ok(Self {
            client: SiteClient::new(config, Some(age))?,
        })
    }
}

#[async_trait]
impl Source for Tapas {
    fn name(&self) -> &'static str {
        SOURCE_NAME
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Manga>> {
        let params = [("q", query.to_string()), ("t", "COMICS".to_string())];
        let html = self.client.get("/search", &params).await?;
        Ok(parse_search(&html).into_iter().take(limit).collect())
    }

    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        let html = self.client.get(&format!("/series/{id}/info"), &[]).await?;
        parse_series(id, &html)
    }

    async fn chapters(&self, id: &MangaId, language: &str) -> Result<Vec<Chapter>> {
        if !INFO.languages.contains(&language) {
            return Ok(Vec::new());
        }
        let path = format!("/series/{id}/episodes");
        let mut episodes = Vec::new();
        let mut page = Some(1);
        while let Some(n) = page {
            let body = self.client.get(&path, &[("page", n.to_string())]).await?;
            let (batch, has_next) = parse_episodes(&body)?;
            page = next_episode_page(id, n, has_next, batch.len(), MAX_EPISODE_PAGES)?;
            episodes.extend(batch);
        }
        let chapters = to_chapters(&episodes, id);
        let locked = chapters.iter().filter(|c| !c.is_available()).count();
        if locked > 0 {
            log::info!("tapas: series {id}: {locked} locked episodes");
        }
        Ok(chapters)
    }

    async fn pages(&self, chapter: &ChapterId) -> Result<PageUrls> {
        let html = self.client.get(&format!("/episode/{chapter}"), &[]).await?;
        parse_pages(chapter, &html)
    }

    fn page_headers(&self, url: &str) -> Vec<Header> {
        self.client.page_headers(url)
    }

    fn page_url(&self, url: &str) -> String {
        self.client.page_url(url)
    }

    async fn status(&self) -> ProviderStatus {
        self.client.status(PROBE).await
    }
}
