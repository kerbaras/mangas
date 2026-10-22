//! Send to Kindle over USB: the Kindles found, the copy with progress, and its cancel.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mangas_core::send::{mtp, usb, KindleVolume, SendVia};
use tauri::{AppHandle, Emitter, State};

use super::send::record;
use super::to_ipc;

/// Event name for USB copy progress; payload is [`CopyProgress`].
pub const SEND_EVENT: &str = "send-progress";

#[derive(Clone, serde::Serialize)]
struct CopyProgress {
    done: u64,
    total: u64,
}

/// Kindles mounted right now.
#[tauri::command]
pub async fn find_kindles() -> Vec<KindleVolume> {
    tauri::async_runtime::spawn_blocking(usb::find_kindles)
        .await
        .unwrap_or_default()
}

/// A Kindle on the USB bus; `copyable` when this build copies to it over MTP (macOS and
/// Windows), else only its name is of use (the format table, Email instead).
#[derive(serde::Serialize)]
pub struct BusKindle {
    name: String,
    copyable: bool,
}

/// Kindles on the USB bus, mounted or not: asked when none is mounted, to tell a Kindle
/// connected over MTP only from no Kindle at all.
#[tauri::command]
pub async fn usb_kindles() -> Vec<BusKindle> {
    let found = tauri::async_runtime::spawn_blocking(usb::usb_kindles).await;
    let copyable = mtp::copyable();
    let to_dto = |k: usb::UsbKindle| BusKindle {
        name: k.name,
        copyable,
    };
    found.unwrap_or_default().into_iter().map(to_dto).collect()
}

/// The cancel flag of the USB copy under way (one at a time).
#[derive(Default, Clone)]
pub struct UsbCopying(Arc<AtomicBool>);

/// Stops the USB copy under way at its next chunk: the partial book is removed and a book
/// already on the Kindle stays; `send_usb` then fails with "copy cancelled".
#[tauri::command]
pub fn cancel_send_usb(copying: State<'_, UsbCopying>) {
    copying.0.store(true, Ordering::SeqCst);
}

/// Copies `file` to the Kindle at `root` (the first one found when `None`) and returns where it
/// is. "Device not found" when there is none. A format the Kindle doesn't open is never copied,
/// one it may not open only with `force` (asked of a Kindle on MTP only too, before any upload).
/// A Kindle on MTP only is copied to directly where [`mtp::copyable`]. A book that was already
/// on the Kindle isn't copied and adds no history row. [`cancel_send_usb`] stops it.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn send_usb(
    app: AppHandle,
    copying: State<'_, UsbCopying>,
    file: PathBuf,
    title: String,
    root: Option<PathBuf>,
    force: Option<bool>,
) -> Result<usb::Copied, String> {
    let cancel = copying.0.clone();
    cancel.store(false, Ordering::SeqCst);
    let (copied, name) = tauri::async_runtime::spawn_blocking({
        let file = file.clone();
        move || {
            let chosen = usb::target(root.as_deref(), usb::scan_kindles());
            if let Some(name) = usb::target_name(&chosen) {
                usb::admit(&usb::usb_verdict(&file, name), force == Some(true))?;
            }
            let kindle = chosen?;
            // One event per 1 MB chunk would repaint the dialog ~30 times a second.
            let mut last = Instant::now() - Duration::from_secs(1);
            let mut on_progress = |done, total| {
                if done == total || last.elapsed() >= Duration::from_millis(250) {
                    last = Instant::now();
                    if let Err(e) = app.emit(SEND_EVENT, CopyProgress { done, total }) {
                        log::warn!("emit {SEND_EVENT}: {e}");
                    }
                }
            };
            let cancelled = || cancel.load(Ordering::SeqCst);
            usb::deliver(&file, &kindle, &cancelled, &mut on_progress)
                .map(|d| (d, kindle.name().to_string()))
        }
    })
    .await
    .map_err(to_ipc)?
    .map_err(to_ipc)?;
    if !copied.already_there {
        record(&file, &title, SendVia::Usb, &name);
    }
    Ok(copied)
}
