//! Device export: e-ink filters and the page pipeline shared by every format.

mod cover;
mod estimate;
mod export;
pub mod filters;
mod input;
mod parts;
mod preview;
pub mod process;
mod request;
mod run;
mod stale;
pub mod trim;

use super::{profile, Profile, PROFILES};

pub use estimate::{estimate, estimate_largest, Estimate};
pub use export::export;
pub use input::{chapter_files, chapter_images, decode, ChapterFiles};
pub use parts::{
    book_path, oversized, part_owner, part_path, Oversized, PART_SIZE, SEND_TO_KINDLE_LIMIT,
};
pub use preview::{preview, Preview, PreviewPage, PreviewRequest};
pub use process::{process_page, EncodedPage, ImageSettings};
pub use request::{BookMeta, ExportRequest, ExportStats, Part, Progress};

/// A target e-reader. Kept as the historic name of [`Profile`]; ids are stored in settings.
pub type Device = Profile;

pub const DEVICES: &[Device] = PROFILES;

pub fn device(id: &str) -> Option<Device> {
    profile(id)
}
