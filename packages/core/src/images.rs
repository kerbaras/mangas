//! Page image formats the exporter can decode: the `image` crate features enabled in the
//! workspace `Cargo.toml`. Export, import and anything else that picks pages share this list.

use std::path::Path;

use image::ImageFormat;

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

/// Extension of an image body from its magic bytes: the [`PAGE_EXTENSIONS`] formats plus `avif`
/// (recognised so the file is named for what it is, even though export skips it). `None` for
/// anything else, an HTML error page served with a 200 say.
pub fn sniff_extension(bytes: &[u8]) -> Option<&'static str> {
    match image::guess_format(bytes).ok()? {
        ImageFormat::Jpeg => Some("jpg"),
        ImageFormat::Png => Some("png"),
        ImageFormat::WebP => Some("webp"),
        ImageFormat::Gif => Some("gif"),
        ImageFormat::Avif => Some("avif"),
        _ => None,
    }
}

/// The shared page check, the same one the cover cache uses: a body in a decodable page format
/// that decodes completely to a non-empty image and, for JPEG, is not cut off. `Ok` is the
/// extension to store it under; `Err` says why the page can't be used. Decoding the whole image
/// catches damaged pixel data that would otherwise fail later, at export.
pub fn check_page(bytes: &[u8]) -> Result<&'static str, String> {
    let ext = sniff_extension(bytes).ok_or("not an image")?;
    if !is_page_extension(ext) {
        return Err(format!("{} images aren't supported", ext.to_uppercase()));
    }
    match image::load_from_memory(bytes) {
        Ok(img) if img.width() > 0 && img.height() > 0 && !truncated_jpeg(ext, bytes) => Ok(ext),
        _ => Err(format!("damaged {} image", ext.to_uppercase())),
    }
}

/// JPEG decoders fill a cut-off scan with grey instead of failing. In a whole file the last scan
/// (start-of-scan marker) is followed by the end-of-image marker; trailing padding is fine.
pub fn truncated_jpeg(ext: &str, bytes: &[u8]) -> bool {
    let last = |marker: [u8; 2]| bytes.windows(2).rposition(|w| w == marker);
    ext == "jpg" && last([0xFF, 0xD9]) <= last([0xFF, 0xDA])
}
