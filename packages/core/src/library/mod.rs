//! Local library: SQLite store of the manga the user follows and the chapters on disk.

mod category_repo;
mod chapter_marks;
mod chapter_repo;
mod counts;
mod db;
mod download_repo;
mod entries;
mod gate_repo;
pub mod import;
mod job_repo;
mod language_sync;
mod languages;
mod manga_repo;
mod reading_status;
mod refresh;
mod settings_repo;
mod snapshot;
mod sync;
mod title_flags;
pub mod updates;

pub use category_repo::{
    clean_category_name, create_category, delete_category, list_categories, rename_category,
    set_manga_categories, Category,
};
pub use chapter_marks::{mark_downloaded, mark_exported};
pub use chapter_repo::{list_chapters, upsert_chapters};
pub use counts::{count_chapters, ChapterCounts, ChapterMark};
pub use db::{open, open_in_memory, Library};
pub(crate) use download_repo::switch_data_dir;
pub use download_repo::{downloaded_chapters, forget_download, DownloadedChapter};
pub use entries::{list_entries, LibraryEntry};
pub use gate_repo::{
    queue_gate, queue_paused, set_queue_held_until, set_queue_paused, take_queue_hold,
};
pub use import::{import_paths, is_local, ImportReport, ImportRequest, LOCAL_SOURCE};
pub use job_repo::{delete_jobs, enqueue_jobs, list_jobs, update_jobs, update_jobs_gated};
pub use languages::{
    in_languages, parse_languages, prefer_languages, requested_languages, validate_languages,
};
pub(crate) use manga_repo::list_manga_with_added;
pub use manga_repo::{get_manga, list_manga, remove_manga, upsert_manga};
pub use reading_status::{set_reading_status, ReadingStatus};
pub use refresh::{refresh_from_source, Refreshed};
pub use settings_repo::{
    get_settings, set_settings, ProviderSession, Settings, DEFAULT_LANGUAGE, DEFAULT_UPDATE_HOURS,
};
pub use snapshot::{
    read_snapshot, restore_snapshot, CategoryState, ChapterState, LibrarySnapshot, TitleState,
};
pub use sync::{fetch_chapters, store_chapters, sync_chapters};
pub use title_flags::{mark_viewed, set_auto_download};
