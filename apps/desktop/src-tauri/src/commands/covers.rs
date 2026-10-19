use std::path::PathBuf;

use mangas_core::covers::{self, CoverCache, Limits};
use mangas_core::domain::Manga;
use mangas_providers::covers::CoverFetcher;

use super::{settings_quietly, to_ipc};
use crate::paths;

/// Local file of a manga's cover, fetched through its provider (and the proxy setting) and
/// cached in the platform data dir. Unreadable settings fall back to the defaults (no warning: the
/// library grid asks for every cover), so a cached cover is still found.
/// The UI loads it through the asset protocol; an error means "show the placeholder".
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn cover_path(source: String, manga_id: String, url: String) -> Result<String, String> {
    let settings = settings_quietly();
    let provider = mangas_providers::source(&source, &settings).map_err(to_ipc)?;
    let fetcher = CoverFetcher::via(provider, &settings.network()).map_err(to_ipc)?;
    let cache = CoverCache::new(paths::covers()?, Limits::default());
    let path = covers::cover(&cache, &fetcher, &source, &manga_id, &url)
        .await
        .map_err(to_ipc)?;
    Ok(path.to_string_lossy().into_owned())
}

/// The manga's cover file for an export, or `None` (the first page becomes the cover) when it
/// has none or it can't be fetched.
pub(super) async fn cover_file(manga: &Manga) -> Option<PathBuf> {
    let url = manga.cover_url.clone()?;
    cover_path(manga.source.clone(), manga.id.0.clone(), url)
        .await
        .inspect_err(|e| tracing::warn!("export without the series cover: {e}"))
        .ok()
        .map(PathBuf::from)
}
