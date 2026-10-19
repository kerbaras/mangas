//! Local library: SQLite store of the manga the user follows and the chapters on disk.

mod category_repo;
mod chapter_marks;
mod chapter_repo;
mod counts;
mod db;
mod download_repo;
mod entries;
mod export_history;
mod export_labels;
mod export_presets;
mod folders;
mod gate_repo;
pub mod import;
mod job_repo;
mod language_sync;
mod languages;
mod manga_repo;
mod reading_status;
mod refresh;
mod settings_merge;
mod settings_prefs;
mod settings_repo;
mod snapshot;
mod stored_releases;
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
pub use export_history::{list_exports, record_export, record_sent, ExportRecord};
pub use export_labels::{chapter_titles, export_labels};
pub use export_presets::{
    clear_export_preset, get_export_preset, preset_for_dirs, series_of_dirs, series_of_spelled,
    set_export_preset,
};
pub use folders::{export_folders, openable_folder};
pub use gate_repo::{
    queue_gate, queue_paused, set_queue_held_until, set_queue_paused, take_queue_hold,
};
pub use import::{import_paths, is_local, ImportReport, ImportRequest, Skipped, LOCAL_SOURCE};
pub use job_repo::{
    delete_job, delete_jobs, enqueue_jobs, list_jobs, update_jobs, update_jobs_gated,
};
pub use languages::{
    in_languages, parse_languages, prefer_languages, requested_languages, validate_languages,
};
pub(crate) use manga_repo::list_manga_with_added;
pub use manga_repo::{get_manga, list_manga, remove_manga, upsert_manga};
pub use reading_status::{set_reading_status, ReadingStatus};
pub use refresh::{refresh_from_source, Refreshed};
pub use settings_merge::merge_edit;
pub use settings_prefs::{
    proxy_url, Network, ProxyMode, Theme, CONCURRENCY_RANGE, DEFAULT_CONCURRENCY, DEFAULT_LANGUAGE,
    DEFAULT_UPDATE_HOURS,
};
pub use settings_repo::{
    edit_settings, get_settings, set_settings, settings_or_default, ProviderSession, Settings,
};
pub use snapshot::{
    read_snapshot, restore_snapshot, CategoryState, ChapterState, LibrarySnapshot, TitleState,
};
pub use sync::{fetch_chapters, store_chapters, sync_chapters};
pub use title_flags::{chapter_group, mark_viewed, set_auto_download, set_chapter_group};
