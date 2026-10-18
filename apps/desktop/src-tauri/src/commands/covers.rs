use mangas_core::covers::{self, CoverCache, Limits};
use mangas_providers::covers::CoverFetcher;

use super::{source_by_name, to_ipc};
use crate::paths;

/// Local file of a manga's cover, fetched through its provider and cached under the data dir.
/// The UI loads it through the asset protocol; an error means "show the placeholder".
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn cover_path(source: String, manga_id: String, url: String) -> Result<String, String> {
    let fetcher = CoverFetcher::for_source(source_by_name(&source)?).map_err(to_ipc)?;
    let cache = CoverCache::new(paths::covers()?, Limits::default());
    let path = covers::cover(&cache, &fetcher, &source, &manga_id, &url)
        .await
        .map_err(to_ipc)?;
    Ok(path.to_string_lossy().into_owned())
}
