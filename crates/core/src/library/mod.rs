//! Local library: SQLite store of the manga the user follows and the chapters on disk.

mod chapter_repo;
mod db;
mod manga_repo;

pub use chapter_repo::{list_chapters, mark_downloaded, upsert_chapters};
pub use db::{open, open_in_memory, Library};
pub use manga_repo::{get_manga, list_manga, remove_manga, upsert_manga};
