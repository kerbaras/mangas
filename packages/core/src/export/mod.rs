//! Export of downloaded chapters: device profiles, the chapter-directories-to-pages pipeline
//! (`kindle`), and one [`Exporter`] per file format (EPUB, CBZ, PDF).

pub mod cbz;
pub mod epub;
mod exporter;
pub mod kindle;
mod output;
pub mod page;
pub mod pdf;
pub mod preset;
mod profiles;
pub mod volume;

pub use exporter::{Exporter, Format, Labels, Meta};
pub use output::{free_names_in, resolve_free_names};
pub use page::PageSettings;
pub use preset::ExportPreset;
pub use profiles::{profile, Profile, PROFILES};
