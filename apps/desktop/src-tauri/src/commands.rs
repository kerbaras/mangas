use std::path::PathBuf;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::download::{self, Event};
use mangas_core::export::kindle::{self, Device, ExportRequest, ExportStats, Progress, DEVICES};
use mangas_core::library::{self, Library};
use mangas_core::ml;
use mangas_core::smartcut::Preset;
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
    library::upsert_chapters(&lib, source.name(), &id, &language, &chapters).map_err(to_ipc)?;
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
        library::upsert_chapters(&lib, src.name(), &id, &language, &fresh).map_err(to_ipc)?;
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

/// Emitted as `kindle-progress` while `export_kindle` runs.
#[derive(Clone, serde::Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum ExportEvent {
    ModelDownload {
        id: String,
        done: u64,
        total: Option<u64>,
    },
    Export(Progress),
}

#[tauri::command]
pub async fn export_kindle(
    app: AppHandle,
    chapter_dirs: Vec<PathBuf>,
    device: String,
    preset: Preset,
    detectors: bool,
    output: PathBuf,
    title: Option<String>,
) -> Result<ExportStats, String> {
    let dev = kindle::device(&device).ok_or_else(|| format!("unknown device '{device}'"))?;
    let penalizer = if detectors {
        let cache = paths::models()?;
        let mut on_progress = |id: &str, done: u64, total: Option<u64>| {
            let _ = app.emit(
                "kindle-progress",
                ExportEvent::ModelDownload {
                    id: id.to_string(),
                    done,
                    total,
                },
            );
        };
        Some(
            ml::row_penalizer(&cache, &mut on_progress)
                .await
                .map_err(to_ipc)?,
        )
    } else {
        None
    };
    let req = ExportRequest {
        title: title.unwrap_or_else(|| default_title(&chapter_dirs)),
        chapter_dirs,
        device: dev,
        cut: preset.options(),
        output,
    };
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut on_progress = |p: Progress| {
            let _ = handle.emit("kindle-progress", ExportEvent::Export(p));
        };
        kindle::export(&req, penalizer, &mut on_progress).map_err(to_ipc)
    })
    .await
    .map_err(to_ipc)?
}

fn default_title(chapter_dirs: &[PathBuf]) -> String {
    chapter_dirs
        .first()
        .and_then(|c| c.parent().and_then(|p| p.file_name()))
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Manga".to_string())
}

#[tauri::command]
pub fn detectors_status() -> Result<ml::Status, String> {
    Ok(ml::status(&paths::models()?))
}
