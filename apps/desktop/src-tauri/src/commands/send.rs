//! Send to Kindle after an export: email, USB copy, test email and the export history.

use std::path::PathBuf;

use mangas_core::library::{self, ExportRecord};
use mangas_core::send::password::{self, Storage};
use mangas_core::send::{email, usb, EmailSettings, KindleVolume, SendVia};
use tauri::{AppHandle, Emitter};

use super::{blocking, open_library, to_ipc};

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

/// Copies `file` to the Kindle at `root` (the first one found when `None`) and returns where it
/// is. "Device not found" when there is none. A book that was already on the Kindle isn't
/// copied and adds no history row.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn send_usb(
    app: AppHandle,
    file: PathBuf,
    title: String,
    root: Option<PathBuf>,
) -> Result<usb::Copied, String> {
    let (copied, name) = tauri::async_runtime::spawn_blocking({
        let file = file.clone();
        move || {
            let kindle = usb::choose(root.as_deref(), usb::find_kindles())?;
            let mut on_progress = |done, total| {
                if let Err(e) = app.emit(SEND_EVENT, CopyProgress { done, total }) {
                    log::warn!("emit {SEND_EVENT}: {e}");
                }
            };
            usb::copy_to_kindle(&file, &kindle, &mut on_progress).map(|d| (d, kindle.name))
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

/// Emails `file` to the Kindle address in the settings. The user asked for it, so a locked
/// keychain prompts to unlock once; dismissed, the send fails with that.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn send_email(file: PathBuf, title: String) -> Result<email::Sent, String> {
    let settings = blocking(|| {
        password::retry_keychain();
        let store = password::system_store();
        password::email_settings(&open_library()?, &*store).map_err(to_ipc)
    })
    .await?;
    let sent = email::send_book(&settings, &file, &title)
        .await
        .map_err(to_ipc)?;
    record(&file, &title, SendVia::Email, &sent.to);
    Ok(sent)
}

/// Sends a test message with `settings` (the form as edited, saved or not: it loads with the
/// keychain's password, so an empty one was cleared); returns the address it went to.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn send_test_email(settings: EmailSettings) -> Result<String, String> {
    email::send_test(&settings).await.map_err(to_ipc)
}

/// The saved SMTP password and where it is kept.
#[derive(serde::Serialize)]
pub struct EmailPassword {
    password: String,
    storage: Storage,
}

/// The saved SMTP password (moved to the keychain first if still in the settings) and where it
/// is kept. Only for something the user did: with `retry`, a locked keychain prompts to unlock
/// even after a dismissed prompt; without, it answers `locked` from memory.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn email_password(retry: bool) -> Result<EmailPassword, String> {
    blocking(move || {
        if retry {
            password::retry_keychain();
        }
        let (lib, store) = (open_library()?, password::system_store());
        let settings = password::load_settings(&lib, &*store).map_err(to_ipc)?;
        Ok(EmailPassword {
            password: settings.kindle_email.password,
            storage: password::storage(&lib, &*store).map_err(to_ipc)?,
        })
    })
    .await
}

/// The `limit` most recent exports, newest first.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn list_exports(limit: usize) -> Result<Vec<ExportRecord>, String> {
    library::list_exports(&open_library()?, limit).map_err(to_ipc)
}

/// The book reached the device either way; a history that can't record it only warns.
fn record(file: &std::path::Path, title: &str, via: SendVia, to: &str) {
    if let Err(e) = open_library()
        .and_then(|lib| library::record_sent(&lib, file, title, via, to).map_err(to_ipc))
    {
        tracing::warn!("could not record the delivery: {e}");
    }
}
