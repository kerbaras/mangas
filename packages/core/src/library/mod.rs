//! Local library: SQLite store of the manga the user follows and the chapters on disk.

mod chapter_repo;
mod db;
mod download_repo;
pub mod import;
mod languages;
mod manga_repo;
mod refresh;
mod settings_repo;
mod sync;

pub use chapter_repo::{list_chapters, mark_downloaded, upsert_chapters};
pub use db::{open, open_in_memory, Library};
pub(crate) use download_repo::switch_data_dir;
pub use download_repo::{downloaded_chapters, forget_download, DownloadedChapter};
pub use import::{import_paths, is_local, ImportReport, ImportRequest, LOCAL_SOURCE};
pub use languages::{
    in_languages, parse_languages, prefer_languages, requested_languages, validate_languages,
};
pub(crate) use manga_repo::list_manga_with_added;
pub use manga_repo::{get_manga, list_manga, remove_manga, upsert_manga};
pub use refresh::{refresh_from_source, Refreshed};
pub use settings_repo::{get_settings, set_settings, ProviderSession, Settings, DEFAULT_LANGUAGE};
pub use sync::{fetch_chapters, store_chapters, sync_chapters};
