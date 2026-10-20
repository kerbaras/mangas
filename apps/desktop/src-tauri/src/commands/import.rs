use std::path::PathBuf;

use mangas_core::library::{self, ImportReport, ImportRequest};

use super::export::hold_for_export;
use super::{open_library, settings_or_default, to_ipc};
use crate::paths;

/// Imports chapter folders / `.cbz` files (or one series folder) as a `local` manga. Pages are
/// copied off the UI thread; the report lists imported and skipped entries.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn import_local(
    paths: Vec<PathBuf>,
    title: Option<String>,
    language: String,
) -> Result<ImportReport, String> {
    let lib = open_library()?;
    // Pages are copied into the data folder: no data-folder move until the import is done.
    let writing = hold_for_export().map_err(to_ipc)?;
    let root = paths::downloads(&settings_or_default()).map_err(to_ipc)?;
    let req = ImportRequest {
        paths,
        title,
        language,
    };
    // Mapped here, not in the closure, so the error is logged under this command's span.
    tauri::async_runtime::spawn_blocking(move || {
        let _writing = writing;
        library::import_paths(&lib, &root, &req)
    })
    .await
    .map_err(to_ipc)?
    .map_err(to_ipc)
}
