//! Pure naming: which file holds a manga's cover.

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

/// `<stem>-<content>.<ext>`: a digest of the stored bytes lets a lookup spot a file that was cut
/// short or overwritten on disk.
pub fn file_name(stem: &str, bytes: &[u8], ext: &str) -> String {
    format!("{stem}-{}.{ext}", content_tag(bytes))
}

/// Short digest of a cover body, as recorded in its [`file_name`].
pub fn content_tag(bytes: &[u8]) -> String {
    hex(bytes)[..16].to_string()
}

/// Content digest a cover file was stored with, read back from its [`file_name`].
pub fn stored_tag(name: &str) -> Option<&str> {
    let (base, _ext) = name.rsplit_once('.')?;
    Some(base.rsplit_once('-')?.1)
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_content_tag_reads_back_from_the_name() {
        let name = file_name(
            &file_stem("comick", "m1", "https://c/a.jpg"),
            b"cover",
            "jpg",
        );
        assert_eq!(stored_tag(&name), Some(content_tag(b"cover").as_str()));
        assert_eq!(stored_tag("garbage"), None);
    }
}
