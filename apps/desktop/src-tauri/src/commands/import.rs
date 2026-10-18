use std::path::PathBuf;

use mangas_core::library::{self, ImportReport, ImportRequest};

use super::{open_library, settings_of, to_ipc};
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
    let root = paths::downloads(&settings_of(&lib)?).map_err(to_ipc)?;
    let req = ImportRequest {
        paths,
        title,
        language,
    };
    // Mapped here, not in the closure, so the error is logged under this command's span.
    tauri::async_runtime::spawn_blocking(move || library::import_paths(&lib, &root, &req))
        .await
        .map_err(to_ipc)?
        .map_err(to_ipc)
}
