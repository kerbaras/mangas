use std::path::PathBuf;

use mangas_core::export::kindle::{self, ExportRequest, ExportStats, Progress};
use mangas_core::library;
use mangas_core::ml;
use mangas_core::smartcut::{Preset, RowPenalizer};
use tauri::{AppHandle, Emitter};

use super::{open_library, settings_of, to_ipc};
use crate::paths;

/// Event name for export progress; payload is [`ExportEvent`].
pub const KINDLE_EVENT: &str = "kindle-progress";

#[derive(Clone, serde::Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub(super) enum ExportEvent {
    ModelDownload {
        id: String,
        done: u64,
        total: Option<u64>,
    },
    Export(Progress),
    /// A volume export starts book `index` of `total`.
    Volume {
        index: usize,
        total: usize,
        title: String,
    },
}

pub(super) fn emit(app: &AppHandle, event: ExportEvent) {
    if let Err(e) = app.emit(KINDLE_EVENT, event) {
        log::warn!("emit {KINDLE_EVENT}: {e}");
    }
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn export_kindle(
    app: AppHandle,
    chapter_dirs: Vec<PathBuf>,
    device: String,
    preset: Preset,
    detectors: bool,
    output: PathBuf,
    title: Option<String>,
) -> Result<ExportStats, String> {
    let dev =
        kindle::device(&device).ok_or_else(|| to_ipc(format!("unknown device '{device}'")))?;
    let penalizer = penalizer(&app, detectors).await?;
    let req = ExportRequest {
        title: title.unwrap_or_else(|| default_title(&chapter_dirs)),
        chapter_dirs,
        device: dev,
        cut: preset.options(),
        output,
        meta: Default::default(),
    };
    let handle = app.clone();
    // Mapped here, not in the closure, so the error is logged under this command's span.
    let (stats, dirs) = tauri::async_runtime::spawn_blocking(move || {
        let mut on_progress = |p: Progress| emit(&handle, ExportEvent::Export(p));
        kindle::export(&req, penalizer, &mut on_progress).map(|s| (s, req.chapter_dirs))
    })
    .await
    .map_err(to_ipc)?
    .map_err(to_ipc)?;
    mark_exported(&stats.exported_chapters(&dirs));
    Ok(stats)
}

/// The detector row penalizer when `detectors` is on, reporting model downloads as events.
pub(super) async fn penalizer(
    app: &AppHandle,
    detectors: bool,
) -> Result<Option<RowPenalizer>, String> {
    if !detectors {
        return Ok(None);
    }
    let cache = models_dir()?;
    let mut on_progress = |id: &str, done: u64, total: Option<u64>| {
        let id = id.to_string();
        emit(app, ExportEvent::ModelDownload { id, done, total });
    };
    let p = ml::row_penalizer(&cache, &mut on_progress)
        .await
        .map_err(to_ipc)?;
    Ok(Some(p))
}

/// The EPUB is written either way; a library that can't record it only loses the "exported" mark.
pub(super) fn mark_exported(dirs: &[&std::path::Path]) {
    if let Err(e) =
        open_library().and_then(|lib| library::mark_exported(&lib, dirs).map_err(to_ipc))
    {
        tracing::warn!("could not mark chapters exported: {e}");
    }
}

fn models_dir() -> Result<PathBuf, String> {
    let lib = open_library()?;
    paths::models(&settings_of(&lib)?).map_err(to_ipc)
}

fn default_title(chapter_dirs: &[PathBuf]) -> String {
    chapter_dirs
        .first()
        .and_then(|c| c.parent().and_then(|p| p.file_name()))
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Manga".to_string())
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn detectors_status() -> Result<ml::Status, String> {
    Ok(ml::status(&models_dir()?))
}
