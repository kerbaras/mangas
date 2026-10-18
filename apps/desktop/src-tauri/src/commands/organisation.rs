//! The user's library organisation: entries with their state, categories and reading status.

use mangas_core::domain::MangaId;
use mangas_core::library::{self, Category, LibraryEntry, ReadingStatus};

use super::{open_library, to_ipc};

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn list_entries() -> Result<Vec<LibraryEntry>, String> {
    library::list_entries(&open_library()?).map_err(to_ipc)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn list_categories() -> Result<Vec<Category>, String> {
    library::list_categories(&open_library()?).map_err(to_ipc)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn create_category(name: String) -> Result<Category, String> {
    library::create_category(&open_library()?, &name).map_err(to_ipc)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn rename_category(id: i64, name: String) -> Result<Category, String> {
    library::rename_category(&open_library()?, id, &name).map_err(to_ipc)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn delete_category(id: i64) -> Result<(), String> {
    library::delete_category(&open_library()?, id).map_err(to_ipc)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn set_manga_categories(
    source: String,
    manga_id: String,
    category_ids: Vec<i64>,
) -> Result<(), String> {
    let id = MangaId(manga_id);
    library::set_manga_categories(&open_library()?, &source, &id, &category_ids).map_err(to_ipc)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn set_reading_status(
    source: String,
    manga_id: String,
    status: ReadingStatus,
) -> Result<(), String> {
    let id = MangaId(manga_id);
    library::set_reading_status(&open_library()?, &source, &id, status).map_err(to_ipc)
}
