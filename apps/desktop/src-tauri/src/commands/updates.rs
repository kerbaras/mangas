//! Update checks (`library::updates`): run one, clear a title's "new" badge, toggle its
//! auto-download.

use mangas_core::domain::MangaId;
use mangas_core::library::updates::{self, Summary, UpdatePolicy, UpdateReport};
use mangas_core::library::{self};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_notification::NotificationExt;

use super::downloads::Downloads;
use super::{errors::to_ipc, open_library, settings_quietly};

/// A check's report with its totals, for the toast.
#[derive(Serialize)]
pub struct UpdateRun {
    #[serde(flatten)]
    report: UpdateReport,
    summary: Summary,
    /// Chapters queued for download (titles with auto-download on).
    queued: usize,
}

/// Checks the followed titles for new chapters. A `background` run (launch, timer) notifies the
/// desktop when something is new, unless notifications are off in the settings. New chapters of
/// auto-download titles go to the download queue.
#[tauri::command]
#[tracing::instrument(level = "error", skip(app, downloads))]
pub async fn check_updates(
    app: AppHandle,
    downloads: State<'_, Downloads>,
    background: bool,
) -> Result<UpdateRun, String> {
    let lib = open_library()?;
    let report = mangas_providers::updates::check_updates(&lib, &UpdatePolicy::default())
        .await
        .map_err(to_ipc)?;
    let queued = queue_new(&downloads, &report)?;
    // Quiet: `check_updates` already warned about a broken settings row.
    if background && settings_quietly().notify_updates() {
        if let Some((title, body)) = updates::notification(&report) {
            notify(&app, &title, &body);
        }
    }
    let summary = updates::summary(&report);
    Ok(UpdateRun {
        report,
        summary,
        queued,
    })
}

fn queue_new(downloads: &Downloads, report: &UpdateReport) -> Result<usize, String> {
    if !report.updates.iter().any(|u| u.auto_download) {
        return Ok(0);
    }
    let queue = downloads.queue()?;
    let jobs = updates::enqueue_new(queue.library(), report, false).map_err(to_ipc)?;
    if !jobs.is_empty() {
        queue.nudge();
    }
    Ok(jobs.len())
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    let shown = app.notification().builder().title(title).body(body).show();
    if let Err(e) = shown {
        tracing::warn!("update notification not shown: {e}");
    }
}

/// The user opened (or left) the title's details: its chapters stop counting as new.
#[tauri::command]
#[tracing::instrument(level = "error")]
pub fn mark_viewed(source: String, manga_id: String) -> Result<bool, String> {
    library::mark_viewed(&open_library()?, &source, &MangaId(manga_id)).map_err(to_ipc)
}

#[tauri::command]
#[tracing::instrument(level = "error")]
pub fn set_auto_download(source: String, manga_id: String, on: bool) -> Result<bool, String> {
    library::set_auto_download(&open_library()?, &source, &MangaId(manga_id), on).map_err(to_ipc)
}

/// Narrows a followed title to one scanlation group; `None` shows every group again.
#[tauri::command]
#[tracing::instrument(level = "error")]
pub fn set_chapter_group(
    source: String,
    manga_id: String,
    group: Option<String>,
) -> Result<bool, String> {
    let lib = open_library()?;
    library::set_chapter_group(&lib, &source, &MangaId(manga_id), group.as_deref()).map_err(to_ipc)
}
