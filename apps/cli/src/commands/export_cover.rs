//! The series cover for `mangas export` books.

use std::path::PathBuf;

use mangas_core::covers::{self, CoverCache, Limits};
use mangas_core::domain::Manga;
use mangas_providers::covers::CoverFetcher;

use super::source_by_name;
use crate::paths;

/// The cached series cover, fetched once if needed; `None` (first page as cover) when offline.
pub(super) async fn cover_file(manga: &Manga) -> Option<PathBuf> {
    let url = manga.cover_url.as_deref()?;
    let cache = CoverCache::new(paths::folders().ok()?.covers, Limits::default());
    let fetcher = source_by_name(&manga.source).and_then(|s| Ok(CoverFetcher::for_source(s)?));
    let fetched = match fetcher {
        Ok(f) => covers::cover(&cache, &f, &manga.source, &manga.id.0, url)
            .await
            .map_err(anyhow::Error::from),
        Err(e) => Err(e),
    };
    fetched
        .inspect_err(|e| eprintln!("warning: no series cover ({e}); the first page is the cover"))
        .ok()
}
