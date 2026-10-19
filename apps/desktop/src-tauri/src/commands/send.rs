//! Send to Kindle after an export: email, USB copy, test email and the export history.

use std::path::PathBuf;
use std::sync::Arc;

use mangas_core::library::{self, ExportRecord};
use mangas_core::send::password::{self, SecretStore, Storage};
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

/// Sends a test message with `settings`, the form as edited (saved or not), and returns the
/// address it went to. Unless the user `typed` a new password it uses the saved one, like a
/// send: a locked keychain prompts to unlock once; dismissed, the test fails with that.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn send_test_email(settings: EmailSettings, typed: bool) -> Result<String, String> {
    test_email(Arc::from(password::system_store()), settings, typed).await
}

async fn test_email(
    store: Arc<dyn SecretStore>,
    draft: EmailSettings,
    typed: bool,
) -> Result<String, String> {
    let settings =
        blocking(move || password::test_settings(&*store, &draft, typed).map_err(to_ipc)).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use mangas_core::send::password::MemoryStore;
    use std::net::TcpListener;

    #[tokio::test]
    async fn a_locked_keychain_fails_the_test_email_before_any_smtp() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        server.set_nonblocking(true).unwrap();
        let draft = EmailSettings {
            to: "reader@kindle.com".into(),
            from: "me@example.test".into(),
            host: "127.0.0.1".into(),
            port: Some(server.local_addr().unwrap().port()),
            username: "me@example.test".into(),
            ..EmailSettings::default()
        };
        let locked = Arc::new(MemoryStore::locked("prompt dismissed"));
        let err = test_email(locked, draft, false).await.unwrap_err();
        assert!(err.contains("keychain is locked"), "{err}");
        assert!(server.accept().is_err(), "connected to SMTP");
    }
    #[tokio::test]
    async fn a_typed_test_email_then_a_save_reach_a_locked_keychain_once() {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let draft = EmailSettings {
            to: "reader@kindle.com".into(),
            host: "127.0.0.1".into(),
            port: Some(port),
            username: "me@example.test".into(),
            password: "typed".into(),
            ..EmailSettings::default()
        };
        let store = Arc::new(MemoryStore::locked("prompt dismissed"));
        let _ = test_email(store.clone(), draft, true).await;
        assert_eq!(store.calls(), 0);
        let lib = library::open_in_memory().unwrap();
        let base = password::load_settings(&lib, &MemoryStore::default()).unwrap();
        let mut edited = base.clone();
        edited.kindle_email.password = "new-secret".into();
        let err = password::save_settings(&lib, &*store, Some(&base), &edited).unwrap_err();
        assert!(err.to_string().contains("keychain is locked"), "{err}");
        assert_eq!(store.calls(), 1);
    }
}
