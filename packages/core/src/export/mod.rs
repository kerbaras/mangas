//! Export of downloaded chapters: device profiles, the chapter-directories-to-pages pipeline
//! (`kindle`), and one [`Exporter`] per file format (EPUB, CBZ, PDF).

pub mod azw3;
pub mod cbz;
pub mod epub;
mod exporter;
pub mod kindle;
mod output;
pub mod page;
mod part_claim;
pub mod pdf;
pub mod preset;
mod profiles;
mod space;
pub(crate) use space::is_out_of_space;
mod spool;
mod spool_file;
pub mod target;
mod unlisted;
pub mod volume;

pub use exporter::{Exporter, Format, Labels, Meta};
pub(crate) use output::write_file;
pub use output::{free_names_in, in_export_folder, resolve_free_names};
pub use page::{default_quality, PageSettings, JPEG_QUALITY};
pub use preset::ExportPreset;
pub use profiles::{profile, Profile, PROFILES};
pub use spool::{PageInfo, PageSpool};
