//! Local library: SQLite store of the manga the user follows and the chapters on disk.

mod db;
mod manga_repo;

pub use db::{open, open_in_memory, Library};
pub use manga_repo::{list_manga, upsert_manga};
