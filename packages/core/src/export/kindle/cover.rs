//! The series cover as a device page.

use std::path::Path;

use super::input::decode;
use super::process::{process_page, ImageSettings};
use crate::export::epub::PageImage;

/// The cover at `path` fitted and filtered like any page, or `None` (with a warning) when it
/// can't be read, so the book falls back to its first page as the cover.
pub(super) fn cover_page(path: &Path, settings: &ImageSettings) -> Option<PageImage> {
    let page = decode(path).and_then(|img| process_page(&img, settings));
    match page {
        Ok(p) => Some(PageImage {
            jpeg: p.jpeg,
            width: p.width,
            height: p.height,
        }),
        Err(e) => {
            tracing::warn!(cover = %path.display(), "export: cover unreadable, using the first page: {e}");
            None
        }
    }
}
