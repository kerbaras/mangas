use std::path::PathBuf;

use mangas_core::export::kindle::{self, BookMeta, ExportRequest, ExportStats, Progress};
use mangas_core::export::{ExportPreset, Format, Labels};
use mangas_core::library;
use mangas_core::smartcut::Preset;
use tauri::{AppHandle, Emitter};

use super::penalizer::penalizer;
use super::{open_library, settings_or_default, to_ipc};
use crate::paths;

/// Event name for export progress; payload is [`ExportEvent`].
pub const EXPORT_EVENT: &str = "export-progress";

#[derive(Clone, serde::Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub(super) enum ExportEvent {
    ModelDownload {
        id: String,
        done: u64,
        total: Option<u64>,
    },
    /// Detector support is missing (deleted, or never downloaded) and downloads first.
    DetectorsMissing {
        bytes: u64,
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
    if let Err(e) = app.emit(EXPORT_EVENT, event) {
        log::warn!("emit {EXPORT_EVENT}: {e}");
    }
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
#[allow(clippy::too_many_arguments)] // Tauri maps each IPC argument to a parameter.
pub async fn export_chapters(
    app: AppHandle,
    chapter_dirs: Vec<PathBuf>,
    device: Option<String>,
    format: Option<Format>,
    preset: Option<Preset>,
    detectors: bool,
    output: PathBuf,
    title: Option<String>,
) -> Result<ExportStats, String> {
    let explicit = ExportPreset {
        cut_preset: preset,
        format,
        profile: device,
        ..Default::default()
    };
    let resolved = resolve(&explicit, series_preset(&chapter_dirs).as_ref())?;
    let dev = resolved
        .profile_or(paths::DEFAULT_PROFILE)
        .map_err(to_ipc)?;
    let format = Format::resolve(format, Some(&output), resolved.format, &dev).map_err(to_ipc)?;
    let device = dev.id.to_string();
    let detection = penalizer(&app, detectors).await?;
    let req_dirs = chapter_dirs.clone();
    let req = ExportRequest {
        labels: labels_of(&chapter_dirs),
        // The desktop always sends a generated title, so the chapter's own title wins.
        title_explicit: false,
        title: title.unwrap_or_else(|| default_title(&chapter_dirs)),
        chapter_dirs,
        device: dev,
        format,
        cut: resolved.cut(),
        page: resolved.page(),
        output,
        meta: BookMeta {
            chapter_titles: titles_of(&req_dirs),
            ..Default::default()
        },
    };
    let title = req.title.clone();
    let handle = app.clone();
    // Mapped here, not in the closure, so the error is logged under this command's span.
    let penalizer = detection.penalizer;
    let (mut stats, dirs) = tauri::async_runtime::spawn_blocking(move || {
        let mut on_progress = |p: Progress| emit(&handle, ExportEvent::Export(p));
        kindle::export(&req, penalizer, &mut on_progress).map(|s| (s, req.chapter_dirs))
    })
    .await
    .map_err(to_ipc)?
    .map_err(to_ipc)?;
    record_export(&stats, &dirs, &title, &device);
    stats.detectors_error = detection.error;
    Ok(stats)
}

/// `explicit` choices over the series preset over the stored defaults (the built-in ones when
/// the settings can't be read).
pub(super) fn resolve(
    explicit: &ExportPreset,
    series: Option<&ExportPreset>,
) -> Result<ExportPreset, String> {
    let fallback = ExportPreset::from_settings(&settings_or_default());
    Ok(ExportPreset::resolve(explicit, series, &fallback))
}

/// The preset of the series every folder belongs to; none when unreadable or mixed.
fn series_preset(dirs: &[PathBuf]) -> Option<ExportPreset> {
    let refs: Vec<&std::path::Path> = dirs.iter().map(PathBuf::as_path).collect();
    open_library()
        .and_then(|lib| library::preset_for_dirs(&lib, &refs).map_err(to_ipc))
        .unwrap_or_else(|e| {
            tracing::warn!("could not read the series export preset: {e}");
            None
        })
}

/// Marks the chapters exported and adds the book to the export history. The file is written
/// either way; a library that can't record it only loses the marks.
pub(super) fn record_export(stats: &ExportStats, dirs: &[PathBuf], title: &str, device: &str) {
    let recorded = open_library().and_then(|lib| {
        library::mark_exported(&lib, &stats.exported_chapters(dirs)).map_err(to_ipc)?;
        library::record_export(&lib, &stats.output, title, device).map_err(to_ipc)
    });
    if let Err(e) = recorded {
        tracing::warn!("could not record the export: {e}");
    }
}

/// Series/chapter/volume labels from the library; without it the file is just unlabelled.
fn labels_of(dirs: &[PathBuf]) -> Labels {
    let refs: Vec<&std::path::Path> = dirs.iter().map(PathBuf::as_path).collect();
    open_library()
        .and_then(|lib| library::export_labels(&lib, &refs, dirs.len()).map_err(to_ipc))
        .unwrap_or_else(|e| {
            tracing::warn!("could not read chapter labels: {e}");
            Labels::default()
        })
}

/// Season-aware TOC entries from the library; empty (folder names) when it doesn't know them.
fn titles_of(dirs: &[PathBuf]) -> Vec<String> {
    let spellings: Vec<Vec<&std::path::Path>> = dirs.iter().map(|d| vec![d.as_path()]).collect();
    open_library()
        .and_then(|lib| library::chapter_titles(&lib, &spellings).map_err(to_ipc))
        .unwrap_or_else(|e| {
            tracing::warn!("could not read chapter titles: {e}");
            Vec::new()
        })
}

fn default_title(chapter_dirs: &[PathBuf]) -> String {
    chapter_dirs
        .first()
        .and_then(|c| c.parent().and_then(|p| p.file_name()))
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Manga".to_string())
}
