//! Page image formats the exporter can decode: the `image` crate features enabled in the
//! workspace `Cargo.toml`. Export, import and anything else that picks pages share this list.

use std::path::Path;

/// Extensions of the formats the enabled decoders read; a GIF decodes as its first frame.
/// AVIF is left out on purpose: decoding it needs `image`'s `avif-native` feature (the dav1d C
/// library), so those files are skipped with a warning instead.
pub const PAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif"];

/// Whether `ext` (without the dot, any case) is a page format we can decode.
pub fn is_page_extension(ext: &str) -> bool {
    PAGE_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext))
}

/// Whether `path` has a decodable page extension.
pub fn has_page_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(is_page_extension)
}
