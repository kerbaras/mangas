//! Detector support (ONNX Runtime + models): status, download with progress and cancel, remove.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use mangas_core::ml;
use tauri::{AppHandle, Emitter};

use super::export::models_dir;
use super::to_ipc;

/// Download progress: `{ item, done, total }` in bytes, `item` is "onnxruntime" or a model id.
pub const DETECTORS_EVENT: &str = "detectors://download";

/// What holds the models folder. Claims, releases and Cancel all change it under this lock, so a
/// Cancel always lands on the download the user saw start, never on the next one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Holder {
    Download,
    Move,
}

static HOLDER: Mutex<Option<Holder>> = Mutex::new(None);
static CANCEL: AtomicBool = AtomicBool::new(false);

fn holder() -> MutexGuard<'static, Option<Holder>> {
    HOLDER.lock().unwrap_or_else(PoisonError::into_inner)
}

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
    let Some(_busy) = claim(Holder::Download) else {
        return Err("detector support is already downloading, or the data folder is moving".into());
    };
    download(&app).await
}

/// Held while a detector download or a data-folder move uses the folder; dropping it frees it.
pub struct Busy;

impl Drop for Busy {
    fn drop(&mut self) {
        let mut holder = holder();
        CANCEL.store(false, Ordering::SeqCst);
        *holder = None;
    }
}

fn claim(kind: Holder) -> Option<Busy> {
    let mut holder = holder();
    if holder.is_some() {
        return None;
    }
    CANCEL.store(false, Ordering::SeqCst);
    *holder = Some(kind);
    Some(Busy)
}

/// None while a download or another move holds the folder.
pub fn claim_for_move() -> Option<Busy> {
    claim(Holder::Move)
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
    // Ignored unless a download holds the folder, so it never aborts a later one.
    if *holder() == Some(Holder::Download) {
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
    use std::net::TcpListener;

    use super::*;

    /// One test: the claim and the Cancel flag are process-wide.
    #[tokio::test]
    async fn cancel_hits_only_the_running_download() {
        let moving = claim_for_move().expect("free at start");
        assert!(
            claim(Holder::Download).is_none(),
            "a move and a download never overlap"
        );
        cancel_detectors_download();
        assert!(
            !CANCEL.load(Ordering::SeqCst),
            "a Cancel during a move is ignored"
        );
        drop(moving);
        cancel_detectors_download();
        assert!(
            !CANCEL.load(Ordering::SeqCst),
            "a Cancel when idle is ignored"
        );

        // A server that accepts and never answers: the Cancel comes before the first chunk.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/ort.tgz", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let _held: Vec<_> = listener.incoming().take(1).collect();
            std::thread::sleep(Duration::from_secs(30));
        });
        let busy = claim(Holder::Download).expect("free after the move");
        let run = tokio::spawn(async move {
            let mut ignore = |_, _| {};
            ml::fetch::fetch(&url, &mut ignore, &CANCEL).await
        });
        cancel_detectors_download();
        let err = tokio::time::timeout(Duration::from_secs(5), run)
            .await
            .expect("Cancel right after start stops the download")
            .unwrap()
            .unwrap_err();
        assert!(err.to_string().contains(ml::fetch::CANCELLED), "{err}");
        drop(busy);
        assert!(
            !CANCEL.load(Ordering::SeqCst),
            "the next download is not pre-cancelled"
        );
        assert!(claim(Holder::Download).is_some(), "freed when dropped");
    }
}
