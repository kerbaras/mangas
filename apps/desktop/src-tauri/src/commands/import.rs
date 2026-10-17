use std::path::PathBuf;

use mangas_core::library::{self, ImportReport, ImportRequest};

use super::{open_library, settings_of, to_ipc};
use crate::paths;

/// Imports chapter folders / `.cbz` files (or one series folder) as a `local` manga. Pages are
/// copied off the UI thread; the report lists imported and skipped entries.
#[tauri::command]
pub async fn import_local(
    paths: Vec<PathBuf>,
    title: Option<String>,
    language: String,
) -> Result<ImportReport, String> {
    let lib = open_library()?;
    let root = paths::downloads(&settings_of(&lib)?)?;
    let req = ImportRequest {
        paths,
        title,
        language,
    };
    tauri::async_runtime::spawn_blocking(move || {
        library::import_paths(&lib, &root, &req).map_err(to_ipc)
    })
    .await
    .map_err(to_ipc)?
}
