//! Cover images, cached on disk: fetched once through the provider (referer, session, rate
//! limit), kept per manga under the data dir and reused across restarts. The cache is bounded;
//! the least recently shown covers go first.

mod cache;
mod check;
mod evict;
mod key;

pub use cache::{CoverCache, Limits};
pub use check::{decoded_extension, image_extension, looks_intact};
pub use evict::{to_evict, Entry};
pub use key::{content_tag, file_name, file_stem, stored_tag};

use std::path::{Path, PathBuf};

use crate::domain::Result;
use crate::download::PageFetcher;

/// The cover cache folder: in the platform data dir next to the library database, so neither the
/// data folder setting nor the defaults used when the settings can't be read move it.
pub fn dir(platform_dir: &Path) -> PathBuf {
    platform_dir.join("covers")
}

/// Local file of the cover at `url` for manga `source`/`manga_id`: the cached copy, or one
/// fetched with `fetcher` and stored. Fetch errors and bodies that are not complete images are
/// errors; nothing is cached for them, so the next call tries again. A cached file that no
/// longer looks intact is dropped and fetched again.
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
