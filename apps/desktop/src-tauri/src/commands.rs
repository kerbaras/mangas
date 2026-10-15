use std::path::PathBuf;

use mangas_core::domain::Manga;
use mangas_core::export::kindle::{self, Device, ExportRequest, ExportStats, Progress, DEVICES};
use mangas_core::library;
use mangas_core::ml;
use mangas_core::smartcut::Preset;
use tauri::{AppHandle, Emitter};

use crate::paths;

/// Errors cross IPC as their display string.
fn to_ipc(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub fn list_library() -> Result<Vec<Manga>, String> {
    let lib = library::open(&paths::library_db().map_err(to_ipc)?).map_err(to_ipc)?;
    library::list_manga(&lib).map_err(to_ipc)
}

#[tauri::command]
pub fn list_devices() -> Vec<Device> {
    DEVICES.to_vec()
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
