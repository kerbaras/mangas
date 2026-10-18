//! Detector support (ONNX Runtime + models): status, download with progress and cancel, remove.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use mangas_core::ml;
use tauri::{AppHandle, Emitter};

use super::export::models_dir;
use super::to_ipc;

/// Download progress: `{ item, done, total }` in bytes, `item` is "onnxruntime" or a model id.
pub const DETECTORS_EVENT: &str = "detectors://download";

static DOWNLOADING: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);

#[derive(Clone, serde::Serialize)]
struct DownloadProgress {
    item: String,
    done: u64,
    total: Option<u64>,
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn detectors_status() -> Result<ml::Status, String> {
    Ok(ml::status(&models_dir()?))
}

/// Downloads whatever is missing, emitting [`DETECTORS_EVENT`] at most ten times a second.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn download_detectors(app: AppHandle) -> Result<ml::Status, String> {
    if DOWNLOADING.swap(true, Ordering::SeqCst) {
        return Err("detector support is already downloading, or the data folder is moving".into());
    }
    let result = download(&app).await;
    // Cleared after, not before: a Cancel sent while the call was on its way still counts.
    CANCEL.store(false, Ordering::SeqCst);
    DOWNLOADING.store(false, Ordering::SeqCst);
    result
}

/// Keeps downloads out of the data folder while it moves; false if one is already running.
pub fn claim() -> bool {
    DOWNLOADING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
}

pub fn release() {
    DOWNLOADING.store(false, Ordering::SeqCst);
}

async fn download(app: &AppHandle) -> Result<ml::Status, String> {
    let dir = models_dir()?;
    let mut last = Instant::now() - Duration::from_secs(1);
    let mut on_progress = |item: &str, done: u64, total: Option<u64>| {
        if Some(done) == total || last.elapsed() >= Duration::from_millis(100) {
            last = Instant::now();
            let p = DownloadProgress {
                item: item.to_string(),
                done,
                total,
            };
            if let Err(e) = app.emit(DETECTORS_EVENT, p) {
                log::warn!("emit {DETECTORS_EVENT}: {e}");
            }
        }
    };
    ml::download(&dir, &mut on_progress, &CANCEL)
        .await
        .map_err(to_ipc)
}

#[tauri::command]
pub fn cancel_detectors_download() {
    // Ignored when idle, so a late Cancel cannot abort the next download.
    if DOWNLOADING.load(Ordering::SeqCst) {
        CANCEL.store(true, Ordering::SeqCst);
    }
}

/// Deletes the runtime and models; returns the bytes freed.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn remove_detectors() -> Result<u64, String> {
    ml::remove(&models_dir()?).map_err(to_ipc)
}
