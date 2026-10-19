use mangas_core::domain::{Chapter, Manga, MangaId};
use mangas_core::library::{self, Refreshed};
use mangas_core::sources::SearchGroup;

use super::{open_library, settings_or_default, source_by_name, to_ipc};

/// Searches the enabled providers (narrowed to `providers` when not empty), one group each. A
/// provider that fails or needs a session reports it in its group.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn search_all(
    query: String,
    limit: usize,
    providers: Vec<String>,
) -> Result<Vec<SearchGroup>, String> {
    let settings = settings_or_default();
    Ok(mangas_providers::search_enabled(&settings, &query, limit, &providers).await)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn follow_manga(source: String, id: String, language: String) -> Result<Manga, String> {
    let language = mangas_providers::language_for(&source, &language);
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
    let language = mangas_providers::language_for(&source, &language);
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
