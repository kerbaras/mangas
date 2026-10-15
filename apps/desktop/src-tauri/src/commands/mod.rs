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
