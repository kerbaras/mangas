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

/// `<stem>-<length>.<ext>`: the stored length lets a lookup spot a file cut short on disk.
pub fn file_name(stem: &str, len: usize, ext: &str) -> String {
    format!("{stem}-{len}.{ext}")
}

/// Length a cover file was stored with, read back from its [`file_name`].
pub fn stored_len(name: &str) -> Option<u64> {
    let (base, _ext) = name.rsplit_once('.')?;
    base.rsplit_once('-')?.1.parse().ok()
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
    fn the_stored_length_reads_back_from_the_name() {
        let name = file_name(&file_stem("comick", "m1", "https://c/a.jpg"), 4096, "jpg");
        assert_eq!(stored_len(&name), Some(4096));
        assert_eq!(stored_len("garbage"), None);
    }
}
