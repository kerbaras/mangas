//! Device export: e-ink filters and the page pipeline shared by every format.

mod cover;
mod export;
pub mod filters;
mod input;
pub mod process;
mod request;

use super::{profile, Profile, PROFILES};

pub use export::export;
pub use input::{chapter_files, chapter_images, decode, ChapterFiles};
pub use process::{process_page, EncodedPage, ImageSettings};
pub use request::{BookMeta, ExportRequest, ExportStats, Progress};

/// A target e-reader. Kept as the historic name of [`Profile`]; ids are stored in settings.
pub type Device = Profile;

pub const DEVICES: &[Device] = PROFILES;

pub fn device(id: &str) -> Option<Device> {
    profile(id)
}
