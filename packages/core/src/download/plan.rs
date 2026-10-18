//! Which pages a chapter download still needs and what their files are called.

use std::collections::HashSet;

use super::cbz::is_image;
use crate::images::sniff_extension;
use crate::sources::PageUrls;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageJob {
    /// 1-based page index.
    pub index: usize,
    pub url: String,
    /// Name before the body is known (extension from the URL); the file is written under
    /// [`page_file_name_for`] once the bytes are in.
    pub file_name: String,
}

/// `0001.png` style file name for page `index` (1-based), keeping the source's extension.
pub fn page_file_name(index: usize, url: &str) -> String {
    format!("{index:04}.{}", extension_of(url))
}

/// File name for page `index` once its bytes are known: the extension follows the image's
/// magic bytes (a JPEG served from a `.png` URL is `0001.jpg`), falling back to the URL's.
pub fn page_file_name_for(index: usize, url: &str, bytes: &[u8]) -> String {
    match sniff_extension(bytes) {
        Some(ext) => format!("{index:04}.{ext}"),
        None => page_file_name(index, url),
    }
}

/// Pages still to fetch given the files already in the chapter directory (resume). A page
/// counts as present by its index (the file stem), whatever its extension: `0003.png` from an
/// older run is page 3 even when the URL now ends in `.jpg` or the bytes turn out to be JPEG,
/// so no duplicate gets written.
pub fn plan(urls: &PageUrls, existing: &HashSet<String>) -> Vec<PageJob> {
    let present = present_indexes(existing);
    urls.0
        .iter()
        .enumerate()
        .map(|(i, url)| PageJob {
            index: i + 1,
            url: url.clone(),
            file_name: page_file_name(i + 1, url),
        })
        .filter(|job| !present.contains(&job.index))
        .collect()
}

/// Indexes of the finished page images among `names` (`0003.png`, not `0003.png.part` or
/// `0003.txt`).
pub(super) fn present_indexes(names: &HashSet<String>) -> HashSet<usize> {
    names
        .iter()
        .filter(|name| is_image(name))
        .filter_map(|name| {
            let (stem, _) = name.rsplit_once('.')?;
            let numbered = !stem.is_empty() && stem.chars().all(|c| c.is_ascii_digit());
            numbered.then(|| stem.parse().ok()).flatten()
        })
        .collect()
}

fn extension_of(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let file = path.rsplit('/').next().unwrap_or(path);
    file.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .filter(|ext| {
            !ext.is_empty() && ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .unwrap_or_else(|| "jpg".to_string())
}
