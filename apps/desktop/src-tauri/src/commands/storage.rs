//! Storage management and library backups: thin wrappers over `mangas_core::{storage, backup}`.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use mangas_core::backup::{self, ExportSummary, Preferences, Restored};
use mangas_core::storage::{
    self, Cache, Cleared, Deleted, Folders, MoveProgress, Moved, PartialCopy, Usage,
};
use tauri::{AppHandle, Emitter};

use super::{open_library, settings_of, to_ipc};
use crate::paths;

/// Progress of [`move_data_dir`]: `{ copied, total }` in bytes.
pub const MOVE_EVENT: &str = "storage://move";

fn folders() -> Result<Folders, String> {
    let settings = settings_of(&open_library()?)?;
    Ok(Folders {
        data_dir: paths::data_dir(&settings)?,
        database: paths::library_db()?,
        covers: paths::covers()?,
        logs: paths::log_dir()?,
    })
}

async fn blocking<T: Send + 'static>(
    job: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(to_ipc)?
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn storage_usage() -> Result<Usage, String> {
    blocking(|| storage::usage(&open_library()?, &folders()?).map_err(to_ipc)).await
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn delete_downloads(source: String, manga_id: String) -> Result<Deleted, String> {
    blocking(move || {
        storage::delete_downloads(&open_library()?, &folders()?.chapters(), &source, &manga_id)
            .map_err(to_ipc)
    })
    .await
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn clear_cache(cache: Cache) -> Result<Cleared, String> {
    blocking(move || storage::clear_cache(&folders()?, cache).map_err(to_ipc)).await
}

/// The partial copy an interrupted move left in `to`, which a move there would replace.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn move_target_partial(to: String) -> Result<Option<PartialCopy>, String> {
    blocking(move || Ok(storage::partial_copy_at(&PathBuf::from(to)))).await
}

/// Deletes the partial copy an interrupted move left (listed in [`storage_usage`]).
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn remove_partial_move() -> Result<Option<PartialCopy>, String> {
    blocking(|| {
        storage::remove_leftover_copy(&open_library()?, &folders()?.data_dir).map_err(to_ipc)
    })
    .await
}

/// Copies the data folder to `to`, verifies it and switches to it, emitting [`MOVE_EVENT`]
/// at most ten times a second. `replace_partial` deletes a partial copy a killed move left there.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn move_data_dir(
    app: AppHandle,
    to: String,
    replace_partial: Option<bool>,
) -> Result<Moved, String> {
    let Some(busy) = super::detectors::claim_for_move() else {
        return Err("detector support is downloading; wait for it or cancel it, then move".into());
    };
    blocking(move || {
        // Held until the copy ends, even if this command is dropped first.
        let _busy = busy;
        let lib = open_library()?;
        let from = folders()?.data_dir;
        let mut last = Instant::now() - Duration::from_secs(1);
        let mut report = |p: MoveProgress| {
            if p.copied == p.total || last.elapsed() >= Duration::from_millis(100) {
                last = Instant::now();
                if let Err(e) = app.emit(MOVE_EVENT, p) {
                    log::warn!("emit {MOVE_EVENT}: {e}");
                }
            }
        };
        let (to, replace) = (PathBuf::from(to), replace_partial.unwrap_or(false));
        storage::move_data_dir_with(&lib, &from, &to, replace, &mut report).map_err(to_ipc)
    })
    .await
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn export_backup(path: String) -> Result<ExportSummary, String> {
    blocking(move || {
        backup::export_to(
            &open_library()?,
            &PathBuf::from(path),
            env!("CARGO_PKG_VERSION"),
        )
        .map_err(to_ipc)
    })
    .await
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn import_backup(
    path: String,
    preferences: Option<Preferences>,
) -> Result<Restored, String> {
    blocking(move || {
        let known: Vec<_> = mangas_providers::providers().iter().map(|p| p.id).collect();
        let (lib, path) = (open_library()?, PathBuf::from(path));
        backup::import_with(&lib, &path, &known, preferences.unwrap_or_default()).map_err(to_ipc)
    })
    .await
}
