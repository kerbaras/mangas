//! Storage management: what the data folders hold, per title and per folder, and the operations
//! that free or move that space. Backups live in [`crate::backup`].

mod caches;
mod delete;
mod layout;
mod move_dir;
mod partial;
mod reading;
mod relocate;
mod siblings;
mod size;
mod tree;
mod usage;

pub use caches::{clear_cache, Cache, Cleared};
pub use delete::{delete_downloads, Deleted};
pub use layout::Folders;
pub use move_dir::{move_data_dir, move_data_dir_with, MoveProgress, Moved};
pub use partial::{leftover_copy, partial_copy_at, remove_leftover_copy, PartialCopy, MOVE_MARKER};
pub use reading::{hold_for_export, EXPORT_LEASE, EXPORT_LEASE_FILES};
pub use relocate::{check_target, rebase};
pub use size::human_size;
pub use tree::dir_size;
pub use usage::{group_by_title, usage, TitleUsage, Usage};
