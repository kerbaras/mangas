//! Settings → Diagnostics: the report behind "Copy diagnostics", "Open logs folder", and a sink
//! for UI errors so crashes in the webview reach the log file too.

use mangas_core::diagnostics::{self, ProviderStatus, Report};
use mangas_providers::PROVIDERS;
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use super::{open_library, settings_of, to_ipc};
use crate::paths;

#[derive(Serialize)]
pub struct Diagnostics {
    pub log_dir: String,
    /// Plain-text report for bug reports.
    pub report: String,
}

#[tauri::command]
pub fn diagnostics() -> Result<Diagnostics, String> {
    let settings = settings_of(&open_library()?)?;
    let log_dir = paths::log_dir()?.to_string_lossy().into_owned();
    let providers = PROVIDERS
        .iter()
        .map(|p| ProviderStatus {
            id: p.id.to_string(),
            status: diagnostics::session_status(
                p.needs_browser_session,
                settings.provider_sessions.get(p.id),
            ),
        })
        .collect();
    let report = Report {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        data_dir: paths::data_dir(&settings)?.to_string_lossy().into_owned(),
        log_dir: log_dir.clone(),
        providers,
    };
    Ok(Diagnostics {
        log_dir,
        report: diagnostics::render(&report),
    })
}

/// Opens the log folder in the system file manager, creating it if logging never could.
#[tauri::command]
pub fn open_logs(app: AppHandle) -> Result<(), String> {
    let dir = paths::log_dir()?;
    std::fs::create_dir_all(&dir).map_err(to_ipc)?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(to_ipc)
}

/// Errors the UI caught (error boundary, unhandled rejections) go to the same log file.
#[tauri::command]
pub fn log_ui_error(message: String) {
    tracing::error!(target: "mangas::ui", "{message}");
}
