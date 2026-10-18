use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::download::{self, Event};
use mangas_core::library::{self, Refreshed};
use mangas_core::sources::SearchGroup;
use mangas_providers::http::PageClient;
use tauri::{AppHandle, Emitter};

use super::{open_library, settings_of, source_by_name, to_ipc};
use crate::paths;

/// Event name the UI listens to for download progress; payload is `download::Event`.
pub const DOWNLOAD_EVENT: &str = "download://progress";

/// Searches the enabled providers (narrowed to `providers` when not empty), one group each. A
/// provider that fails or needs a session reports it in its group; the call itself only fails
/// when the settings cannot be read.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn search_all(
    query: String,
    limit: usize,
    providers: Vec<String>,
) -> Result<Vec<SearchGroup>, String> {
    let settings = settings_of(&open_library()?)?;
    Ok(mangas_providers::search_enabled(&settings, &query, limit, &providers).await)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn follow_manga(source: String, id: String, language: String) -> Result<Manga, String> {
    let source = source_by_name(&source)?;
    let id = MangaId(id);
    let manga = source.manga(&id).await.map_err(to_ipc)?;
    let chapters = source.chapters(&id, &language).await.map_err(to_ipc)?;
    let lib = open_library()?;
    library::upsert_manga(&lib, &manga).map_err(to_ipc)?;
    library::upsert_chapters(&lib, source.name(), &id, &language, &chapters).map_err(to_ipc)?;
    Ok(manga)
}

/// The stored record of a followed manga, otherwise the source's full details (not saved).
/// Search results can be sparse, so the details screen never relies on them alone.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn get_manga(source: String, id: String) -> Result<Manga, String> {
    let id = MangaId(id);
    if let Some(stored) = library::get_manga(&open_library()?, &source, &id).map_err(to_ipc)? {
        return Ok(stored);
    }
    source_by_name(&source)?.manga(&id).await.map_err(to_ipc)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn unfollow_manga(source: String, id: String) -> Result<bool, String> {
    library::remove_manga(&open_library()?, &source, &MangaId(id)).map_err(to_ipc)
}

/// Chapters from the library; `refresh` re-syncs them from the source first. A manga that is not
/// followed yet has nothing stored, so a refresh returns the source's list without saving it.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn list_chapters(
    source: String,
    manga_id: String,
    language: String,
    refresh: bool,
) -> Result<Vec<Chapter>, String> {
    let id = MangaId(manga_id);
    let lib = open_library()?;
    // Imported manga have no provider to refresh from; the library is the only list.
    if refresh && !library::is_local(&source) {
        let src = source_by_name(&source)?;
        let refreshed = library::refresh_from_source(&lib, src.as_ref(), &id, &language)
            .await
            .map_err(to_ipc)?;
        if let Refreshed::NotFollowed(fresh) = refreshed {
            return Ok(fresh);
        }
    }
    // Local chapters keep the language they were imported with (`mangas import --lang`).
    let language = (!library::is_local(&source)).then_some(language.as_str());
    library::list_chapters(&lib, &source, &id, language).map_err(to_ipc)
}

/// Downloads the given chapters, emitting [`DOWNLOAD_EVENT`] as pages and chapters finish.
/// Downloading a manga that is not followed yet follows it first (chapters are stored per manga);
/// a followed manga whose chapter list never got saved is re-synced the same way.
/// Returns the chapter ids that failed (empty on full success).
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn download_chapters(
    app: AppHandle,
    source: String,
    manga_id: String,
    language: String,
    chapter_ids: Vec<String>,
) -> Result<Vec<String>, String> {
    let src = source_by_name(&source)?;
    let lib = open_library()?;
    let id = MangaId(manga_id);
    ensure_followed(&lib, src.as_ref(), &id, &language).await?;
    let known = library::list_chapters(&lib, &source, &id, None).map_err(to_ipc)?;
    let wanted: Vec<ChapterId> = chapter_ids.into_iter().map(ChapterId).collect();
    let selected: Vec<Chapter> = known
        .into_iter()
        .filter(|c| wanted.contains(&c.id))
        .collect();
    // Ids the source no longer lists are reported as failed instead of vanishing quietly.
    let missing: Vec<String> = wanted
        .iter()
        .filter(|id| !selected.iter().any(|c| &c.id == *id))
        .map(|id| id.0.clone())
        .collect();
    let root = paths::downloads(&settings_of(&lib)?).map_err(to_ipc)?;
    let emit = |event: Event| {
        if let Err(e) = app.emit(DOWNLOAD_EVENT, &event) {
            log::warn!("emit {DOWNLOAD_EVENT}: {e}");
        }
    };
    for id in &missing {
        emit(Event::Error {
            chapter: ChapterId(id.clone()),
            message: "not found: the source no longer lists this chapter".into(),
        });
    }
    let pages = PageClient::for_source(src.as_ref()).map_err(to_ipc)?;
    let report = download::download_chapters(src.as_ref(), &pages, &lib, &selected, &root, &emit)
        .await
        .map_err(to_ipc)?;
    let failed = report.failed.into_iter().map(|(id, _)| id.0);
    Ok(missing.into_iter().chain(failed).collect())
}

async fn ensure_followed(
    lib: &library::Library,
    src: &dyn mangas_core::sources::Source,
    id: &MangaId,
    language: &str,
) -> Result<(), String> {
    let followed = library::get_manga(lib, src.name(), id)
        .map_err(to_ipc)?
        .is_some();
    let has_chapters = library::list_chapters(lib, src.name(), id, Some(language))
        .map_err(to_ipc)?
        .iter()
        .any(|c| c.language == language);
    if followed && has_chapters {
        return Ok(());
    }
    let manga = src.manga(id).await.map_err(to_ipc)?;
    let chapters = src.chapters(id, language).await.map_err(to_ipc)?;
    library::upsert_manga(lib, &manga).map_err(to_ipc)?;
    library::upsert_chapters(lib, src.name(), id, language, &chapters).map_err(to_ipc)
}
