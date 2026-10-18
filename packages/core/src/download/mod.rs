//! Chapter downloads: fetch page images from a [`crate::sources::Source`] into a chapter
//! directory with bounded concurrency and resume, reporting progress through [`Event`].

mod availability;
mod cbz;
mod chapter;
mod event;
mod fetcher;
mod incomplete;
mod mirror;
mod pages;
mod selection;

pub use availability::{ensure_available, skipped_summary, split_available};
pub use cbz::{cbz_path, write_cbz};
pub use chapter::{
    chapter_dir, download_chapter_with, download_chapters, unique_chapter_dir, Report,
};
pub use event::{Event, OnEvent};
pub use fetcher::{is_public_http_url, PageFetcher};
pub use mirror::{MirrorFor, WithMirror};
pub use pages::{download_pages, page_file_name, plan, PageJob, CONCURRENCY};
pub use selection::{parse_selection, select_chapters};
