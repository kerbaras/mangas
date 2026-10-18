//! Pure checks that a body is a whole, usable cover image.

use image::ImageFormat;

use super::key::content_tag;

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

/// Check of a cached file before it is served: its bytes are still the ones that were decoded
/// and stored (same content digest), so any damage on disk is a miss.
pub fn looks_intact(bytes: &[u8], stored_tag: &str) -> bool {
    content_tag(bytes) == stored_tag
}

/// JPEG decoders fill a cut-off scan with grey instead of failing. In a whole file the last scan
/// (start-of-scan marker) is followed by the end-of-image marker; trailing padding is fine.
fn truncated_jpeg(ext: &str, bytes: &[u8]) -> bool {
    let last = |marker: [u8; 2]| bytes.windows(2).rposition(|w| w == marker);
    ext == "jpg" && last([0xFF, 0xD9]) <= last([0xFF, 0xDA])
}
