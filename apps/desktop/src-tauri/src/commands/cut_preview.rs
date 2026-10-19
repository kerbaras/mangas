//! Series export presets and the smart-cut preview shown while tuning them.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use mangas_core::export::kindle::{self, Preview, PreviewRequest};
use mangas_core::export::ExportPreset;
use mangas_core::library;

use super::penalizer::penalizer;
use super::preview_panels::{valid_panel, Panels};
use super::{open_library, settings_or_default, to_ipc};
use crate::paths;

/// Pages a preview renders at most.
const PREVIEW_PAGES: usize = 12;
/// Numbers previews, so each writes its own folder.
static GENERATION: AtomicU64 = AtomicU64::new(0);
static PANELS: Mutex<Option<Panels>> = Mutex::new(None);

fn with_panels<T>(f: impl FnOnce(&mut Panels) -> T) -> T {
    let mut panels = PANELS.lock().unwrap_or_else(PoisonError::into_inner);
    f(panels.get_or_insert_with(Panels::default))
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn get_export_preset(source: String, manga_id: String) -> Result<Option<ExportPreset>, String> {
    library::get_export_preset(&open_library()?, &source, &manga_id).map_err(to_ipc)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn set_export_preset(
    source: String,
    manga_id: String,
    preset: ExportPreset,
) -> Result<(), String> {
    library::set_export_preset(&open_library()?, &source, &manga_id, &preset).map_err(to_ipc)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn clear_export_preset(source: String, manga_id: String) -> Result<bool, String> {
    library::clear_export_preset(&open_library()?, &source, &manga_id).map_err(to_ipc)
}

/// The first pages an export of `chapter_dir` with `options` would write (unset fields use the
/// stored defaults), as files under the preview cache. A newer call from the same `panel`, or
/// closing it, cancels this one.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn preview_cut(
    app: tauri::AppHandle,
    chapter_dir: PathBuf,
    options: ExportPreset,
    detectors: bool,
    panel: String,
) -> Result<Preview, String> {
    if !valid_panel(&panel) {
        return Err(format!("invalid preview panel id {panel:?}"));
    }
    let started = with_panels(|p| {
        let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        p.start(&panel, generation).then_some(generation)
    });
    let Some(generation) = started else {
        return Err("cancelled: the options panel closed".into());
    };
    let settings = settings_or_default();
    let preset = options.over(&ExportPreset::from_settings(&settings));
    let device = preset.profile_or(paths::DEFAULT_PROFILE).map_err(to_ipc)?;
    let root = paths::cut_previews().map_err(to_ipc)?.join(&panel);
    let req = PreviewRequest {
        chapter_dir,
        device,
        cut: preset.cut(),
        page: preset.page(),
        limit: PREVIEW_PAGES,
        out_dir: root.join(generation.to_string()),
    };
    let detection = penalizer(&app, detectors, &settings).await?;
    let penalizer = detection.penalizer;
    let watched = panel.clone();
    let cut = tauri::async_runtime::spawn_blocking(move || {
        let stale = || !with_panels(|p| p.is_current(&watched, generation));
        kindle::preview(&req, penalizer, &stale)
    })
    .await
    .map_err(to_ipc)?;
    // A preview that outlived its panel may have written pages after the close removed them.
    if !with_panels(|p| p.is_open(&panel)) {
        remove_dir(&root);
    }
    let mut preview = cut.map_err(to_ipc)?;
    remove_older(&root, generation);
    preview.detectors_error = detection.error;
    Ok(preview)
}

/// Stops the previews of `panel`, which closed: the running one and any that arrive after this.
#[tauri::command]
pub async fn cancel_cut_preview(panel: String) {
    if let (true, Ok(root)) = (valid_panel(&panel), paths::cut_previews()) {
        let dir = root.join(&panel);
        if let Err(e) = tauri::async_runtime::spawn_blocking(move || remove_dir(&dir)).await {
            tracing::debug!("closed preview {panel}: {e}");
        }
    }
    with_panels(|p| p.close(panel));
}

fn remove_dir(dir: &Path) {
    if let Err(e) = std::fs::remove_dir_all(dir) {
        tracing::debug!("closed preview {}: {e}", dir.display());
    }
}

/// Deletes the page folders of the panel's earlier previews; it only shows the latest.
fn remove_older(root: &Path, generation: u64) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let older = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u64>().ok())
            .is_some_and(|g| g < generation);
        if older {
            if let Err(e) = std::fs::remove_dir_all(entry.path()) {
                tracing::debug!("stale preview {}: {e}", entry.path().display());
            }
        }
    }
}
