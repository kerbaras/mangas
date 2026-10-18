//! WeebCentral source (<https://weebcentral.com>): no API, so every step scrapes one HTML page
//! or HTMX fragment with the shared [`crate::html`] helper. One request per `Source` call.

mod chapters;
mod numbering;
mod pages;
mod search;
mod series;

use std::sync::Arc;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Error, Manga, MangaId, Result};
use mangas_core::library::Settings;
use mangas_core::sources::{Header, PageUrls, ProviderStatus, Source};

pub use chapters::{parse_chapters, parse_label, Label};
pub use numbering::{fill_gaps, number_chapters};
pub use pages::parse_pages;
pub use search::{cover_url, parse_search};
pub use series::parse_series;

use crate::http::{ClientConfig, SiteClient};
use crate::provider::{Capability, Provider, ProviderInfo};
use crate::session::Session;

pub const SOURCE_NAME: &str = "weebcentral";
pub const SITE: &str = "https://weebcentral.com";
const REQUESTS_PER_SECOND: u32 = 2;
/// Results per page of the site's own search.
const SEARCH_MAX: usize = 32;
/// The cheapest page that goes through the same stack as a search: one result.
const PROBE: &str = "/search/data?display_mode=Minimal+Display&limit=1&offset=0&text=";

pub const INFO: ProviderInfo = ProviderInfo {
    id: SOURCE_NAME,
    name: "WeebCentral",
    site: SITE,
    languages: &["en"],
    capabilities: &[
        Capability::Search,
        Capability::Chapters,
        Capability::Download,
    ],
    needs_browser_session: false,
    icon_url: Some("https://weebcentral.com/favicon.ico"),
};

pub struct WeebCentralProvider;

impl Provider for WeebCentralProvider {
    fn info(&self) -> ProviderInfo {
        INFO
    }

    fn build(&self, settings: &Settings) -> Result<Arc<dyn Source>> {
        self.build_at(settings, SITE)
    }

    fn build_at(&self, settings: &Settings, base: &str) -> Result<Arc<dyn Source>> {
        let config = ClientConfig {
            requests_per_second: REQUESTS_PER_SECOND,
            // Pages come from rotating CDN hosts; send the referer a browser would send.
            image_hosts: &["*"],
            ..ClientConfig::new(INFO, base)
        };
        let session = Session::from_settings(settings, INFO.id);
        Ok(Arc::new(WeebCentral {
            client: SiteClient::new(config, session)?,
        }))
    }
}

pub struct WeebCentral {
    client: SiteClient,
}

#[async_trait]
impl Source for WeebCentral {
    fn name(&self) -> &'static str {
        SOURCE_NAME
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Manga>> {
        let limit = limit.clamp(1, SEARCH_MAX);
        let params = [
            ("display_mode", "Minimal Display".to_string()),
            ("limit", limit.to_string()),
            ("offset", "0".to_string()),
            ("text", query.trim().to_string()),
            ("sort", "Best Match".to_string()),
            ("order", "Descending".to_string()),
        ];
        let body = self.client.get("/search/data", &params).await?;
        let mut found = parse_search(&body)?;
        found.truncate(limit);
        Ok(found)
    }

    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        let body = self
            .client
            .get(&format!("/series/{}", ulid(&id.0)?), &[])
            .await?;
        parse_series(id, &body)
    }

    async fn chapters(&self, id: &MangaId, language: &str) -> Result<Vec<Chapter>> {
        if !INFO.languages.contains(&language) {
            return Ok(Vec::new());
        }
        let path = format!("/series/{}/full-chapter-list", ulid(&id.0)?);
        parse_chapters(&self.client.get(&path, &[]).await?, id)
    }

    async fn pages(&self, chapter: &ChapterId) -> Result<PageUrls> {
        let params = [
            ("is_prev", "False".to_string()),
            ("current_page", "1".to_string()),
            ("reading_style", "long_strip".to_string()),
        ];
        let path = format!("/chapters/{}/images", ulid(&chapter.0)?);
        let pages = parse_pages(&self.client.get(&path, &params).await?)?;
        if pages.0.is_empty() {
            return Err(Error::NotFound(format!("pages of chapter {chapter}")));
        }
        Ok(pages)
    }

    fn page_headers(&self, url: &str) -> Vec<Header> {
        self.client.page_headers(url)
    }

    async fn status(&self) -> ProviderStatus {
        self.client.status(PROBE).await
    }
}

/// Series and chapter ids are ULIDs; anything else never reaches a URL path.
fn ulid(id: &str) -> Result<&str> {
    if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(id)
    } else {
        Err(Error::NotFound(format!("{SOURCE_NAME} id {id:?}")))
    }
}
