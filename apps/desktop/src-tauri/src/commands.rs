use mangas_core::domain::Manga;
use mangas_core::export::kindle::{Device, DEVICES};
use mangas_core::library;

use crate::paths;

/// Errors cross IPC as their display string.
fn to_ipc(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub fn list_library() -> Result<Vec<Manga>, String> {
    let lib = library::open(&paths::library_db().map_err(to_ipc)?).map_err(to_ipc)?;
    library::list_manga(&lib).map_err(to_ipc)
}

#[tauri::command]
pub fn list_devices() -> Vec<Device> {
    DEVICES.to_vec()
}
