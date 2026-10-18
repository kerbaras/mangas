//! Pure naming: which file holds a manga's cover, and what kind of image a body is.

use image::ImageFormat;
use sha2::{Digest, Sha256};

/// `<manga>-<url>`: one prefix per manga (so a new cover URL replaces the old file) and a tag
/// of the URL (so a changed cover is fetched again).
pub fn file_stem(source: &str, manga_id: &str, url: &str) -> String {
    format!(
        "{}-{}",
        manga_prefix(source, manga_id),
        &hex(url.as_bytes())[..12]
    )
}

/// Prefix shared by every file of one manga's cover.
pub fn manga_prefix(source: &str, manga_id: &str) -> String {
    hex(format!("{source}\0{manga_id}").as_bytes())[..24].to_string()
}

/// File extension for an image body, from its magic bytes; `None` for anything that is not a
/// supported image (an HTML error page served with a 200, a truncated body).
pub fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    match image::guess_format(bytes).ok()? {
        ImageFormat::Jpeg => Some("jpg"),
        ImageFormat::Png => Some("png"),
        ImageFormat::WebP => Some("webp"),
        ImageFormat::Gif => Some("gif"),
        ImageFormat::Avif => Some("avif"),
        _ => None,
    }
}

/// Extensions [`image_extension`] can return, for lookups.
pub const EXTENSIONS: [&str; 5] = ["jpg", "png", "webp", "gif", "avif"];

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
