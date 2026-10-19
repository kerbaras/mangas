//! IPC surface. Each command parses its arguments, calls `mangas_core`, and returns JSON.

pub mod app_update;
#[cfg(all(test, target_os = "linux"))]
mod broken_row_tests;
pub mod catalog;
pub mod covers;
pub mod cut_preview;
pub mod detectors;
pub mod diagnostics;
pub mod downloads;
mod errors;
pub mod export;
pub mod folders;
pub mod import;
pub mod organisation;
mod penalizer;
mod preview_panels;
pub mod providers;
pub mod send;
pub mod smtp_password;
pub mod storage;
pub mod updates;
pub mod volumes;

use mangas_core::domain::Manga;
use mangas_core::export::{Profile, PROFILES};
use mangas_core::library::{self, Library, Settings};
use mangas_core::send::password;
use std::sync::Arc;

use mangas_providers::Source;

use crate::paths;
use errors::to_ipc;

pub(super) fn open_library() -> Result<Library, String> {
    library::open(&paths::library_db().map_err(to_ipc)?).map_err(to_ipc)
}

/// The provider called `name`, carrying the browser session stored for it in the settings.
pub(super) fn source_by_name(name: &str) -> Result<Arc<dyn Source>, String> {
    let settings = settings_of(&open_library()?)?;
    mangas_providers::source(name, &settings).map_err(to_ipc)
}

pub(super) fn settings_of(lib: &Library) -> Result<Settings, String> {
    library::get_settings(lib).map_err(to_ipc)
}

/// The stored settings, or the defaults with a logged warning when they can't be read.
pub(super) fn settings_or_default() -> Settings {
    let (settings, warning) = match paths::library_db() {
        Ok(db) => library::settings_or_default(&db),
        Err(e) => (Settings::default(), Some(e)),
    };
    if let Some(warning) = warning {
        log::warn!("{warning}");
    }
    settings
}

/// The stored settings, or the defaults without a warning: for previews that run on every keystroke.
pub(super) fn settings_quietly() -> Settings {
    paths::library_db()
        .map(|db| library::settings_or_default(&db).0)
        .unwrap_or_default()
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn list_library() -> Result<Vec<Manga>, String> {
    library::list_manga(&open_library()?).map_err(to_ipc)
}

#[tauri::command]
pub fn list_devices() -> Vec<Profile> {
    PROFILES.to_vec()
}

/// Platform data folder used for chapters and models when no `data_dir` setting is stored.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn default_data_dir() -> Result<String, String> {
    Ok(paths::data_dir(&Settings::default())
        .map_err(to_ipc)?
        .to_string_lossy()
        .into_owned())
}

/// The settings as stored. The window reloads them each time it is focused, so this never
/// reads the keychain (a locked one would prompt to unlock every time): a password kept there
/// comes empty, and the email settings ask for it with `send::email_password`.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn get_settings() -> Result<serde_json::Value, String> {
    blocking(|| {
        let lib = open_library()?;
        password::public_settings(&lib, &settings_of(&lib)?).map_err(to_ipc)
    })
    .await
}

/// Preferences edited in the UI on a copy loaded as `base`, saved over whatever another
/// process stored since (`library::edit_settings`); returns what is stored now. Provider
/// sessions are owned by `connect_provider` / `disconnect_provider` and never replaced here.
/// The SMTP password is never part of it (`send::set_smtp_password` saves that); both are
/// returned without it, as `get_settings` does.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn set_settings(
    settings: Settings,
    base: Option<Settings>,
) -> Result<serde_json::Value, String> {
    blocking(move || {
        let (lib, store) = (open_library()?, password::system_store());
        let saved =
            password::save_preferences(&lib, &*store, base.as_ref(), &settings).map_err(to_ipc)?;
        password::public_settings(&lib, &saved.settings).map_err(to_ipc)
    })
    .await
}

pub(super) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(to_ipc)?
}

/// Links this executable as `mangas` on the user's PATH (macOS/Linux). Returns the link path,
/// or the `ln -s` command to run by hand when the directory is not writable.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn install_cli() -> Result<String, String> {
    // Inside an AppImage `current_exe` is the temporary mount; `APPIMAGE` is the real file.
    let exe = match std::env::var_os("APPIMAGE") {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_exe().map_err(to_ipc)?,
    };
    link_cli(&exe).map_err(to_ipc)
}

#[cfg(unix)]
fn link_cli(exe: &std::path::Path) -> Result<String, String> {
    let link = std::path::Path::new("/usr/local/bin/mangas");
    match std::fs::read_link(link) {
        Ok(target) if is_ours(&target) => {
            let _ = std::fs::remove_file(link);
        }
        Ok(target) => {
            return Err(format!(
                "{} already points to {}; remove it first",
                link.display(),
                target.display()
            ))
        }
        Err(_) if link.exists() => {
            return Err(format!(
                "{} exists and is not a link; refusing to replace it",
                link.display()
            ))
        }
        Err(_) => {}
    }
    std::os::unix::fs::symlink(exe, link)
        .map(|()| link.display().to_string())
        .map_err(|e| {
            format!(
                "could not write {}: {e}. Run: sudo ln -sf \"{}\" {}",
                link.display(),
                exe.display(),
                link.display()
            )
        })
}

/// A previous install of ours: dangling, the `mangas` binary, or a Mangas AppImage.
#[cfg(unix)]
fn is_ours(target: &std::path::Path) -> bool {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    !target.exists()
        || name == "mangas"
        || (name.starts_with("mangas") && name.ends_with(".appimage"))
}

#[cfg(not(unix))]
fn link_cli(exe: &std::path::Path) -> Result<String, String> {
    Err(format!(
        "add the folder of {} to your PATH to use the command line",
        exe.display()
    ))
}
