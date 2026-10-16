use async_trait::async_trait;

use crate::domain::{Chapter, ChapterId, Manga, MangaId, Result};

/// Image URLs of one chapter, in reading order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageUrls(pub Vec<String>);

/// One HTTP header (name, value) a source wants on page image requests.
pub type Header = (String, String);

/// A manga provider. Implementations are pure adapters over the provider's API: they map
/// responses to domain types and do not persist anything.
#[async_trait]
pub trait Source: Send + Sync {
    /// Short stable name stored with every manga (e.g. "mangadex").
    fn name(&self) -> &'static str;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Manga>>;
    async fn manga(&self, id: &MangaId) -> Result<Manga>;
    async fn chapters(&self, id: &MangaId, language: &str) -> Result<Vec<Chapter>>;
    async fn pages(&self, chapter: &ChapterId) -> Result<PageUrls>;
    /// Headers the page downloader must send when fetching `url` (referer, browser session
    /// cookie, ...). Implementations keep credentials to their own host.
    fn page_headers(&self, url: &str) -> Vec<Header> {
        let _ = url;
        Vec::new()
    }
}
