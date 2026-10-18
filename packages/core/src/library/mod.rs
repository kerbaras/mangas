//! Local library: SQLite store of the manga the user follows and the chapters on disk.

mod chapter_repo;
mod db;
pub mod import;
mod languages;
mod manga_repo;
mod settings_repo;
mod sync;

pub use chapter_repo::{list_chapters, mark_downloaded, upsert_chapters};
pub use db::{open, open_in_memory, Library};
pub use import::{import_paths, is_local, ImportReport, ImportRequest, LOCAL_SOURCE};
pub use languages::{in_languages, parse_languages, prefer_languages, requested_languages};
pub use manga_repo::{get_manga, list_manga, remove_manga, upsert_manga};
pub use settings_repo::{get_settings, set_settings, ProviderSession, Settings, DEFAULT_LANGUAGE};
pub use sync::{fetch_chapters, store_chapters, sync_chapters};
