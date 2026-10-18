//! Download queue IPC: enqueue chapters, list jobs, pause/resume/cancel/retry one job (`id`)
//! or all of them (`id` = null), clear finished ones. Changes arrive as [`QUEUE_EVENT`].

use std::sync::Arc;

use mangas_core::domain::{Chapter, ChapterId, MangaId};
use mangas_core::download::queue::{self, Action, GateStatus, Job, JobId, Queue, Target};
use mangas_core::library::{self, Library};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use super::{source_by_name, to_ipc};

/// Event the UI listens to; payload is `mangas_core::download::queue::QueueEvent`.
pub const QUEUE_EVENT: &str = "download://queue";

/// The app's queue worker, or why it could not start (unreadable library).
pub struct Downloads(pub Result<Arc<Queue>, String>);

impl Downloads {
    pub(super) fn queue(&self) -> Result<&Arc<Queue>, String> {
        self.0
            .as_ref()
            .map_err(|e| to_ipc(format!("download queue unavailable: {e}")))
    }
}

/// What a download request did: `queued` chapters, ids the source no longer lists (nothing
/// was queued for them) and chapters skipped because they are on disk (`force` re-downloads).
#[derive(Debug, Serialize)]
pub struct DownloadReport {
    pub queued: usize,
    pub missing: Vec<String>,
    pub on_disk: Vec<String>,
}

/// Queues the given chapters. Downloading a manga that is not followed yet follows it first
/// (chapters are stored per manga); a followed manga whose chapter list never got saved is
/// re-synced the same way. Progress reports through the queue events.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn download_chapters(
    downloads: State<'_, Downloads>,
    source: String,
    manga_id: String,
    language: String,
    chapter_ids: Vec<String>,
    force: Option<bool>,
) -> Result<DownloadReport, String> {
    let queue = downloads.queue()?;
    let lib = queue.library();
    let src = source_by_name(&source)?;
    let id = MangaId(manga_id);
    let language = mangas_providers::language_for(&source, &language);
    ensure_followed(lib, src.as_ref(), &id, &language).await?;
    let known = library::list_chapters(lib, &source, &id, None).map_err(to_ipc)?;
    let wanted: Vec<ChapterId> = chapter_ids.into_iter().map(ChapterId).collect();
    let selected: Vec<Chapter> = known
        .into_iter()
        .filter(|c| wanted.contains(&c.id))
        .collect();
    let missing = wanted
        .iter()
        .filter(|id| !selected.iter().any(|c| &c.id == *id))
        .map(|id| id.0.clone())
        .collect();
    let enqueued =
        queue::enqueue(lib, &source, selected, force.unwrap_or(false)).map_err(to_ipc)?;
    queue.nudge();
    Ok(DownloadReport {
        queued: enqueued.jobs.len(),
        missing,
        on_disk: enqueued.on_disk.into_iter().map(|c| c.id.0).collect(),
    })
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn list_downloads(downloads: State<'_, Downloads>) -> Result<Vec<Job>, String> {
    queue::list(downloads.queue()?.library()).map_err(to_ipc)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn pause_download(
    downloads: State<'_, Downloads>,
    id: Option<JobId>,
) -> Result<Vec<Job>, String> {
    control(&downloads, id, Action::Pause)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn resume_download(
    downloads: State<'_, Downloads>,
    id: Option<JobId>,
) -> Result<Vec<Job>, String> {
    control(&downloads, id, Action::Resume)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn cancel_download(
    downloads: State<'_, Downloads>,
    id: Option<JobId>,
) -> Result<Vec<Job>, String> {
    control(&downloads, id, Action::Cancel)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn retry_download(
    downloads: State<'_, Downloads>,
    id: Option<JobId>,
) -> Result<Vec<Job>, String> {
    control(&downloads, id, Action::Retry)
}

/// Removes finished and cancelled jobs from the list; returns how many.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn clear_downloads(downloads: State<'_, Downloads>) -> Result<usize, String> {
    let queue = downloads.queue()?;
    let removed = queue::clear_finished(queue.library()).map_err(to_ipc)?;
    queue.nudge();
    Ok(removed)
}

/// Removes one done, failed or cancelled job (and a chapter row kept only for it).
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn dismiss_download(downloads: State<'_, Downloads>, id: JobId) -> Result<(), String> {
    let queue = downloads.queue()?;
    queue::dismiss(queue.library(), id).map_err(to_ipc)?;
    queue.nudge();
    Ok(())
}

/// The queue-wide pause and data-folder-move hold; later changes arrive as `gate` events.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn queue_status(downloads: State<'_, Downloads>) -> Result<GateStatus, String> {
    queue::gate_status(downloads.queue()?.library()).map_err(to_ipc)
}

/// Opens the chapter folder of job `id` in the file manager.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn open_download_folder(
    app: AppHandle,
    downloads: State<'_, Downloads>,
    id: JobId,
) -> Result<(), String> {
    let job = queue::find(downloads.queue()?.library(), id).map_err(to_ipc)?;
    let dir = job
        .local_path
        .ok_or_else(|| to_ipc(format!("chapter {} is not on disk", job.number)))?;
    app.opener().open_path(dir, None::<&str>).map_err(to_ipc)
}

fn control(downloads: &Downloads, id: Option<JobId>, action: Action) -> Result<Vec<Job>, String> {
    let queue = downloads.queue()?;
    let target = id.map_or(Target::All, Target::Job);
    let changed = queue::control(queue.library(), target, action).map_err(to_ipc)?;
    queue.nudge();
    Ok(changed)
}

async fn ensure_followed(
    lib: &Library,
    src: &dyn mangas_core::sources::Source,
    id: &MangaId,
    language: &str,
) -> Result<(), String> {
    let followed = library::get_manga(lib, src.name(), id)
        .map_err(to_ipc)?
        .is_some();
    let has_chapters = library::list_chapters(lib, src.name(), id, Some(language))
        .map_err(to_ipc)?
        .iter()
        .any(|c| c.language == language);
    if followed && has_chapters {
        return Ok(());
    }
    let manga = src.manga(id).await.map_err(to_ipc)?;
    let chapters = src.chapters(id, language).await.map_err(to_ipc)?;
    library::upsert_manga(lib, &manga).map_err(to_ipc)?;
    library::upsert_chapters(lib, src.name(), id, language, &chapters).map_err(to_ipc)
}
