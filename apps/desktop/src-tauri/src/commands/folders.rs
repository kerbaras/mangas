//! Opening folders in the file manager, limited to the app's own folders (see `openable_folder`).

use std::path::PathBuf;

use mangas_core::library;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use super::{open_library, settings_of, to_ipc};
use crate::paths;

/// Opens `path` if it is a folder inside the app data folder or the data folder from Settings,
/// or exactly a folder the app exported to.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn open_folder(app: AppHandle, path: PathBuf) -> Result<(), String> {
    let lib = open_library()?;
    let roots = [
        paths::default_data_dir()?,
        paths::data_dir(&settings_of(&lib)?)?,
    ];
    let exports = library::export_folders(&lib).map_err(to_ipc)?;
    let dir = library::openable_folder(&path, &roots, &exports).map_err(to_ipc)?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(to_ipc)
}
