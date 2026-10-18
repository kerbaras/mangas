//! Local library: SQLite store of the manga the user follows and the chapters on disk.

mod chapter_repo;
mod db;
pub mod import;
mod manga_repo;
mod refresh;
mod settings_repo;

pub use chapter_repo::{list_chapters, mark_downloaded, upsert_chapters};
pub use db::{open, open_in_memory, Library};
pub use import::{import_paths, is_local, ImportReport, ImportRequest, LOCAL_SOURCE};
pub use manga_repo::{get_manga, list_manga, remove_manga, upsert_manga};
pub use refresh::{refresh_from_source, Refreshed};
pub use settings_repo::{get_settings, set_settings, ProviderSession, Settings, DEFAULT_LANGUAGE};
