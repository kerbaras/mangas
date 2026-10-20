//! MANGA Plus by SHUEISHA (<https://mangaplus.shueisha.co.jp>): the publisher's free
//! simulpub site. Its API answers protobuf ([`proto`], decoded by [`response`]); every request
//! needs a `SESSION-TOKEN` header (any UUID: without one it answers "Account Banned"). Only a
//! title's first and latest chapters are free. Page images need the viewer's token and are
//! XOR-encrypted with a per-page key; both travel in the page URL ([`page`]).

mod mapping;
mod page;
mod proto;
mod response;

use std::sync::Arc;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, Result};
use mangas_core::library::Settings;
use mangas_core::sources::{Header, PageUrls, ProviderStatus, Source};

pub use mapping::{
    chapter_number, language_code, parse_chapters, parse_search, parse_title, parse_viewer,
};
pub use page::{decrypt, page_url, view_token, xor, VIEW_TOKEN_HEADER};

use crate::http::{ClientConfig, SiteClient};
use crate::languages::resolve_languages;
use crate::provider::{Capability, Provider, ProviderInfo};

pub const SOURCE_NAME: &str = "mangaplus";
pub const SITE: &str = "https://mangaplus.shueisha.co.jp";
pub const API_BASE: &str = "https://jumpg-webapi.tokyo-cdn.com/api";
/// One Piece: a title that has been on the site since it opened.
const PROBE: &str = "/title_detailV3?title_id=100020&clang=eng";

pub const INFO: ProviderInfo = ProviderInfo {
    id: SOURCE_NAME,
    name: "MANGA Plus",
    site: SITE,
    languages: &["en", "es", "fr", "id", "pt-br", "ru", "th", "de", "vi"],
    capabilities: &[
        Capability::Search,
        Capability::Chapters,
        Capability::Download,
    ],
    needs_browser_session: false,
    icon_url: Some("https://mangaplus.shueisha.co.jp/favicon.ico"),
};

pub struct MangaPlusProvider;

impl Provider for MangaPlusProvider {
    fn info(&self) -> ProviderInfo {
        INFO
    }

    fn build(&self, settings: &Settings) -> Result<Arc<dyn Source>> {
        Ok(Arc::new(MangaPlus::at(API_BASE, settings)?))
    }

    fn build_at(&self, settings: &Settings, base: &str) -> Result<Arc<dyn Source>> {
        Ok(Arc::new(MangaPlus::at(base, settings)?))
    }
}

pub struct MangaPlus {
    api: SiteClient,
    /// Preferred languages MANGA Plus publishes in; search keeps titles in these.
    languages: Vec<String>,
}

impl MangaPlus {
    pub fn at(base: &str, settings: &Settings) -> Result<Self> {
        let token = uuid::Uuid::new_v4().to_string();
        let config = ClientConfig {
            requests_per_second: 2,
            network: settings.network(),
            headers: vec![("SESSION-TOKEN".to_string(), token)],
            ..ClientConfig::new(INFO, base)
        };
        let languages = resolve_languages(&settings.languages_for(INFO.id), INFO.languages);
        Ok(Self {
            api: SiteClient::new(config, None)?,
            languages: match languages.is_empty() {
                true => vec!["en".to_string()],
                false => languages,
            },
        })
    }

    async fn detail(&self, id: &MangaId) -> Result<Vec<u8>> {
        let query = [("title_id", id.0.clone()), ("clang", "eng".to_string())];
        self.api.get_bytes("/title_detailV3", &query).await
    }
}

#[async_trait]
impl Source for MangaPlus {
    fn name(&self) -> &'static str {
        SOURCE_NAME
    }

    /// The API has no search: the site filters its full title list, and so does this.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Manga>> {
        let body = self.api.get_bytes("/title_list/allV2", &[]).await?;
        parse_search(&body, query, &self.languages, limit)
    }

    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        parse_title(&self.detail(id).await?, id)
    }

    async fn chapters(&self, id: &MangaId, language: &str) -> Result<Vec<Chapter>> {
        parse_chapters(&self.detail(id).await?, id, language)
    }

    async fn pages(&self, chapter: &ChapterId) -> Result<PageUrls> {
        let query = [
            ("chapter_id", chapter.0.clone()),
            ("split", "yes".to_string()),
            ("img_quality", "super_high".to_string()),
            ("clang", "eng".to_string()),
        ];
        parse_viewer(&self.api.get_bytes("/manga_viewer_v3", &query).await?)
    }

    fn page_headers(&self, url: &str) -> Vec<Header> {
        let mut headers = self.api.page_headers(url);
        if let Some(token) = view_token(url) {
            headers.push((VIEW_TOKEN_HEADER.to_string(), token.to_string()));
        }
        headers
    }

    fn page_url(&self, url: &str) -> String {
        self.api.page_url(url)
    }

    fn decode_page(&self, url: &str, bytes: Vec<u8>) -> Result<Vec<u8>> {
        decrypt(url, bytes)
    }

    async fn status(&self) -> ProviderStatus {
        self.api.status(PROBE).await
    }
}
