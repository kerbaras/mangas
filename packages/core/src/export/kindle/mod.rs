//! Device export: e-ink filters and the page pipeline shared by every format.

mod budget;
mod cover;
mod estimate;
mod export;
pub mod filters;
mod input;
mod levers;
mod margins;
mod parts;
mod plan;
mod png_rgba;
mod pool;
mod preview;
pub mod process;
mod request;
mod run;
mod stale;
pub mod trim;

use super::{profile, Profile, PROFILES};

pub use budget::{
    plan_budget, BookBudget, BudgetSpec, Delivery, Levers, QUALITY_STEPS, SEND_TO_KINDLE_WEB_LIMIT,
};
pub use estimate::{estimate, estimate_each, estimate_largest, Estimate};
pub use export::export;
pub use input::{chapter_files, chapter_images, decode, ChapterFiles};
pub use levers::book_levers;
pub use parts::{
    free_book_path, oversized, page_cost, part_book, part_owner, part_path, split_points,
    Oversized, PART_SIZE, SEND_TO_KINDLE_LIMIT,
};
pub use preview::{preview, Preview, PreviewPage, PreviewRequest};
pub use process::{process_page, EncodedPage, ImageSettings};
pub use request::{BookMeta, ExportRequest, ExportStats, Part, Progress};
pub use stale::check_export_folder;

/// A target e-reader. Kept as the historic name of [`Profile`]; ids are stored in settings.
pub type Device = Profile;

pub const DEVICES: &[Device] = PROFILES;

pub fn device(id: &str) -> Option<Device> {
    profile(id)
}
