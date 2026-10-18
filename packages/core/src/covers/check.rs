//! Pure checks that a body is a whole, usable cover image.

use std::io::Cursor;

use image::{ImageFormat, ImageReader};

/// File extension for an image body, from its magic bytes; `None` for anything that is not a
/// supported image (an HTML error page served with a 200, say).
pub fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    match image::guess_format(bytes).ok()? {
        ImageFormat::Jpeg => Some("jpg"),
        ImageFormat::Png => Some("png"),
        ImageFormat::WebP => Some("webp"),
        ImageFormat::Gif => Some("gif"),
        _ => None,
    }
}

/// Extension for a body that decodes completely to a non-empty image. Truncated downloads and
/// corrupt bodies are `None`, so they are never written to the cache.
pub fn decoded_extension(bytes: &[u8]) -> Option<&'static str> {
    let ext = image_extension(bytes)?;
    let image = image::load_from_memory(bytes).ok()?;
    (image.width() > 0 && image.height() > 0 && !truncated_jpeg(ext, bytes)).then_some(ext)
}

/// Cheap check of a cached file before it is served: still the length it was stored with, and
/// its header still describes a non-empty image.
pub fn looks_intact(bytes: &[u8], stored_len: u64) -> bool {
    bytes.len() as u64 == stored_len
        && ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .ok()
            .and_then(|r| r.into_dimensions().ok())
            .is_some_and(|(w, h)| w > 0 && h > 0)
}

/// JPEG decoders fill a cut-off scan with grey instead of failing; a whole file ends with the
/// end-of-image marker (allowing a little trailing padding).
fn truncated_jpeg(ext: &str, bytes: &[u8]) -> bool {
    let tail = &bytes[bytes.len().saturating_sub(64)..];
    ext == "jpg" && !tail.windows(2).any(|w| w == [0xFF, 0xD9])
}
