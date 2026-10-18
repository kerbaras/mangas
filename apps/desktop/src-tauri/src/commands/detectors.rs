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
    let Some(_busy) = claim() else {
        return Err("detector support is already downloading, or the data folder is moving".into());
    };
    download(&app).await
}

/// Held while a detector download or a data-folder move uses the folder; dropping it frees the
/// folder and clears a Cancel sent meanwhile (cleared after, not before, so an early Cancel counts).
pub struct Busy;

impl Drop for Busy {
    fn drop(&mut self) {
        CANCEL.store(false, Ordering::SeqCst);
        DOWNLOADING.store(false, Ordering::SeqCst);
    }
}

/// None while a download or a move already holds the folder.
pub fn claim() -> Option<Busy> {
    DOWNLOADING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .ok()
        .map(|_| Busy)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_claim_excludes_others_and_its_drop_clears_a_cancel() {
        let busy = claim().expect("free at start");
        assert!(claim().is_none(), "a move and a download never overlap");
        cancel_detectors_download();
        assert!(CANCEL.load(Ordering::SeqCst));
        drop(busy);
        assert!(
            !CANCEL.load(Ordering::SeqCst),
            "the next download is not pre-cancelled"
        );
        assert!(claim().is_some(), "freed when dropped");
    }
}
