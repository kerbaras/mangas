//! IPC surface. Each command parses its arguments, calls `mangas_core`, and returns JSON.

pub mod catalog;
pub mod kindle;

use mangas_core::domain::Manga;
use mangas_core::export::kindle::{Device, DEVICES};
use mangas_core::library::{self, Library, Settings};
use mangas_core::sources::{self, Source};

use crate::paths;

/// Errors cross IPC as their display string.
pub(super) fn to_ipc(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub(super) fn open_library() -> Result<Library, String> {
    library::open(&paths::library_db()?).map_err(to_ipc)
}

pub(super) fn source_by_name(name: &str) -> Result<Box<dyn Source>, String> {
    sources::by_name(name).ok_or_else(|| format!("unknown source '{name}'"))
}

pub(super) fn settings_of(lib: &Library) -> Result<Settings, String> {
    library::get_settings(lib).map_err(to_ipc)
}

#[tauri::command]
pub fn list_library() -> Result<Vec<Manga>, String> {
    library::list_manga(&open_library()?).map_err(to_ipc)
}

#[tauri::command]
pub fn list_devices() -> Vec<Device> {
    DEVICES.to_vec()
}

/// Platform data folder used for chapters and models when no `data_dir` setting is stored.
#[tauri::command]
pub fn default_data_dir() -> Result<String, String> {
    Ok(paths::data_dir(&Settings::default())?
        .to_string_lossy()
        .into_owned())
}

#[tauri::command]
pub fn get_settings() -> Result<Settings, String> {
    settings_of(&open_library()?)
}

#[tauri::command]
pub fn set_settings(settings: Settings) -> Result<(), String> {
    library::set_settings(&open_library()?, &settings).map_err(to_ipc)
}

/// Links this executable as `mangas` on the user's PATH (macOS/Linux). Returns the link path,
/// or the `ln -s` command to run by hand when the directory is not writable.
#[tauri::command]
pub fn install_cli() -> Result<String, String> {
    // Inside an AppImage `current_exe` is the temporary mount; `APPIMAGE` is the real file.
    let exe = match std::env::var_os("APPIMAGE") {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_exe().map_err(to_ipc)?,
    };
    link_cli(&exe)
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

/// A previous install of ours: dangling, or a file whose name starts with "mangas".
#[cfg(unix)]
fn is_ours(target: &std::path::Path) -> bool {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    !target.exists() || name.starts_with("mangas")
}

#[cfg(not(unix))]
fn link_cli(exe: &std::path::Path) -> Result<String, String> {
    Err(format!(
        "add the folder of {} to your PATH to use the command line",
        exe.display()
    ))
}
