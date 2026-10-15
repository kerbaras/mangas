use std::path::PathBuf;

use mangas_core::export::kindle::{self, ExportRequest, ExportStats, Progress};
use mangas_core::ml;
use mangas_core::smartcut::Preset;
use tauri::{AppHandle, Emitter};

use super::{open_library, settings_of, to_ipc};
use crate::paths;

/// Event name for export progress; payload is [`ExportEvent`].
pub const KINDLE_EVENT: &str = "kindle-progress";

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

fn emit(app: &AppHandle, event: ExportEvent) {
    if let Err(e) = app.emit(KINDLE_EVENT, event) {
        log::warn!("emit {KINDLE_EVENT}: {e}");
    }
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
        let cache = models_dir()?;
        let mut on_progress = |id: &str, done: u64, total: Option<u64>| {
            let id = id.to_string();
            emit(&app, ExportEvent::ModelDownload { id, done, total });
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
        let mut on_progress = |p: Progress| emit(&handle, ExportEvent::Export(p));
        kindle::export(&req, penalizer, &mut on_progress).map_err(to_ipc)
    })
    .await
    .map_err(to_ipc)?
}

fn models_dir() -> Result<PathBuf, String> {
    let lib = open_library()?;
    paths::models(&settings_of(&lib)?)
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
    Ok(ml::status(&models_dir()?))
}
