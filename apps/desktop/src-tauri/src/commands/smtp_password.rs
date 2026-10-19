//! The SMTP password from the webview's side: whether one is saved and where, and a write-only
//! way to set it. The password itself never goes back to the webview.

use mangas_core::library;
use mangas_core::send::password::{self, SecretStore, Storage};

use super::{blocking, open_library, to_ipc};

/// Whether an SMTP password is saved and where, never the password itself. `fallback`: why a
/// password just saved went to the settings file instead of the keychain.
#[derive(serde::Serialize)]
pub struct PasswordStatus {
    has_password: bool,
    storage: Storage,
    fallback: Option<String>,
}

fn status(lib: &library::Library, store: &dyn SecretStore) -> Result<PasswordStatus, String> {
    let storage = password::storage(lib, store).map_err(to_ipc)?;
    let settings = library::get_settings(lib).map_err(to_ipc)?;
    let has_password = password::has_password(lib, &settings).map_err(to_ipc)?;
    Ok(PasswordStatus {
        has_password,
        storage,
        fallback: None,
    })
}

/// Whether a password is saved and where (moved to the keychain first if still in the
/// settings). Only for something the user did: with `retry`, a locked keychain prompts to
/// unlock even after a dismissed prompt; without, it answers `locked` from memory.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn smtp_password_status(retry: bool) -> Result<PasswordStatus, String> {
    blocking(move || {
        if retry {
            password::retry_keychain();
        }
        let (lib, store) = (open_library()?, password::system_store());
        password::load_settings(&lib, &*store).map_err(to_ipc)?;
        status(&lib, &*store)
    })
    .await
}

/// Saves the SMTP password (empty: clears it) in the keychain, or the settings file without
/// one, and answers where it went; the password is never sent back.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn set_smtp_password(password: String) -> Result<PasswordStatus, String> {
    blocking(move || {
        let (lib, store) = (open_library()?, password::system_store());
        let saved = password::set_password(&lib, &*store, &password).map_err(to_ipc)?;
        Ok(PasswordStatus {
            fallback: saved.fallback,
            ..status(&lib, &*store)?
        })
    })
    .await
}
