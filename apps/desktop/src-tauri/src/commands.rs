use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::download::{self, Event};
use mangas_core::export::kindle::{Device, DEVICES};
use mangas_core::library::{self, Library};
use mangas_core::sources::{self, Source};
use tauri::{AppHandle, Emitter};

use crate::paths;

/// Event name the UI listens to for download progress; payload is `download::Event`.
pub const DOWNLOAD_EVENT: &str = "download://progress";

/// Errors cross IPC as their display string.
fn to_ipc(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn open_library() -> Result<Library, String> {
    library::open(&paths::library_db().map_err(to_ipc)?).map_err(to_ipc)
}

fn source_by_name(name: &str) -> Result<Box<dyn Source>, String> {
    sources::by_name(name).ok_or_else(|| format!("unknown source '{name}'"))
}

#[tauri::command]
pub fn list_library() -> Result<Vec<Manga>, String> {
    library::list_manga(&open_library()?).map_err(to_ipc)
}

#[tauri::command]
pub fn list_devices() -> Vec<Device> {
    DEVICES.to_vec()
}

#[tauri::command]
pub async fn search_manga(
    source: String,
    query: String,
    limit: usize,
) -> Result<Vec<Manga>, String> {
    let source = source_by_name(&source)?;
    source.search(&query, limit).await.map_err(to_ipc)
}

#[tauri::command]
pub async fn follow_manga(source: String, id: String, language: String) -> Result<Manga, String> {
    let source = source_by_name(&source)?;
    let id = MangaId(id);
    let manga = source.manga(&id).await.map_err(to_ipc)?;
    let chapters = source.chapters(&id, &language).await.map_err(to_ipc)?;
    let lib = open_library()?;
    library::upsert_manga(&lib, &manga).map_err(to_ipc)?;
    library::upsert_chapters(&lib, source.name(), &chapters).map_err(to_ipc)?;
    Ok(manga)
}

#[tauri::command]
pub fn unfollow_manga(source: String, id: String) -> Result<bool, String> {
    library::remove_manga(&open_library()?, &source, &MangaId(id)).map_err(to_ipc)
}

/// Chapters of a followed manga from the library; `refresh` re-syncs them from the source first.
#[tauri::command]
pub async fn list_chapters(
    source: String,
    manga_id: String,
    language: String,
    refresh: bool,
) -> Result<Vec<Chapter>, String> {
    let id = MangaId(manga_id);
    let lib = open_library()?;
    if refresh {
        let src = source_by_name(&source)?;
        let fresh = src.chapters(&id, &language).await.map_err(to_ipc)?;
        library::upsert_chapters(&lib, src.name(), &fresh).map_err(to_ipc)?;
    }
    library::list_chapters(&lib, &source, &id, Some(&language)).map_err(to_ipc)
}

/// Downloads the given chapters, emitting [`DOWNLOAD_EVENT`] as pages and chapters finish.
/// Returns the chapter ids that failed (empty on full success).
#[tauri::command]
pub async fn download_chapters(
    app: AppHandle,
    source: String,
    manga_id: String,
    chapter_ids: Vec<String>,
) -> Result<Vec<String>, String> {
    let src = source_by_name(&source)?;
    let lib = open_library()?;
    let known = library::list_chapters(&lib, &source, &MangaId(manga_id), None).map_err(to_ipc)?;
    let wanted: Vec<ChapterId> = chapter_ids.into_iter().map(ChapterId).collect();
    let selected: Vec<Chapter> = known
        .into_iter()
        .filter(|c| wanted.contains(&c.id))
        .collect();
    let root = paths::downloads()?;
    let emit = |event: Event| {
        if let Err(e) = app.emit(DOWNLOAD_EVENT, &event) {
            log::warn!("emit {DOWNLOAD_EVENT}: {e}");
        }
    };
    let report = download::download_chapters(src.as_ref(), &lib, &selected, &root, &emit)
        .await
        .map_err(to_ipc)?;
    Ok(report.failed.into_iter().map(|(id, _)| id.0).collect())
}
