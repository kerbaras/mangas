//! Settings → Diagnostics: the report behind "Copy diagnostics", "Open logs folder", and a sink
//! for UI errors so crashes in the webview reach the log file too.

use mangas_core::diagnostics::{self, ProviderStatus, Report};
use serde::Serialize;

use super::{settings_or_default, to_ipc};
use crate::paths;

#[derive(Serialize)]
pub struct Diagnostics {
    pub log_dir: String,
    /// Plain-text report for bug reports.
    pub report: String,
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn diagnostics() -> Result<Diagnostics, String> {
    let settings = settings_or_default();
    // "Open logs folder" only opens existing folders, and logging may never have created it.
    let data = paths::default_data_dir()?;
    let log_dir = mangas_core::logging::ensure_log_dir(&data)
        .unwrap_or_else(|e| {
            tracing::warn!("could not create the log folder: {e}");
            mangas_core::logging::log_dir(&data)
        })
        .to_string_lossy()
        .into_owned();
    let providers = mangas_providers::providers()
        .into_iter()
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
        data_dir: paths::data_dir(&settings)
            .map_err(to_ipc)?
            .to_string_lossy()
            .into_owned(),
        log_dir: log_dir.clone(),
        providers,
    };
    Ok(Diagnostics {
        log_dir,
        report: diagnostics::render(&report),
    })
}

/// Errors the UI caught (error boundary, unhandled rejections) go to the same log file, and so
/// do recoverable failures (`level: "warn"`, e.g. one chapter that didn't download).
#[tauri::command]
pub fn log_ui_error(message: String, level: Option<String>) {
    match level.as_deref() {
        Some("warn") => tracing::warn!(target: "mangas::ui", "{message}"),
        _ => tracing::error!(target: "mangas::ui", "{message}"),
    }
}
