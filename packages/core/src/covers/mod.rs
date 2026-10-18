//! Cover images, cached on disk: fetched once through the provider (referer, session, rate
//! limit), kept per manga under the data dir and reused across restarts. The cache is bounded;
//! the least recently shown covers go first.

mod cache;
mod evict;
mod key;

pub use cache::{CoverCache, Limits};
pub use evict::{to_evict, Entry};
pub use key::{file_stem, image_extension};

use std::path::PathBuf;

use crate::domain::Result;
use crate::download::PageFetcher;

/// Local file of the cover at `url` for manga `source`/`manga_id`: the cached copy, or one
/// fetched with `fetcher` and stored. Fetch errors and non-image bodies are errors; nothing is
/// cached for them, so the next call tries again.
pub async fn cover(
    cache: &CoverCache,
    fetcher: &dyn PageFetcher,
    source: &str,
    manga_id: &str,
    url: &str,
) -> Result<PathBuf> {
    if let Some(path) = cache.lookup(source, manga_id, url) {
        return Ok(path);
    }
    let bytes = fetcher.fetch(url).await?;
    cache.store(source, manga_id, url, &bytes)
}
