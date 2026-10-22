//! Eject for a mounted Kindle: flush the books just copied and remove the drive safely.

use std::path::PathBuf;

use mangas_core::send::eject::{self, Ejected};
use mangas_core::send::usb;

use super::to_ipc;

/// Flushes `files` (the books just copied, if any) and ejects the Kindle mounted at `root`.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn eject_kindle(root: PathBuf, files: Vec<PathBuf>) -> Result<Ejected, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let kindle = usb::choose(Some(&root), usb::scan_kindles())?;
        eject::eject(&kindle, &files)
    })
    .await
    .map_err(to_ipc)?
    .map_err(to_ipc)
}
