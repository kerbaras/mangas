//! Writes the pages of one entry into a chapter directory as `0001.<ext>`, `0002.<ext>`, ...
//! in natural order, so the exporter's name sort reads them in reading order.

use std::fs::File;
use std::io;
use std::path::Path;

use super::names::{extension, is_page_image, natural_cmp};
use super::scan::{name_of, Entry, EntryKind};
use crate::domain::{Error, Result};

/// Replaces `dest` with the pages of `entry`; returns how many were written.
pub fn write_pages(entry: &Entry, dest: &Path) -> Result<usize> {
    if dest.exists() {
        std::fs::remove_dir_all(dest)?;
    }
    std::fs::create_dir_all(dest)?;
    match entry.kind {
        EntryKind::Folder => link_folder(&entry.path, dest),
        EntryKind::Cbz => extract_cbz(&entry.path, dest),
    }
}

fn page_name(index: usize, original: &str) -> String {
    let ext = extension(original).unwrap_or_else(|| "jpg".into());
    format!("{:04}.{ext}", index + 1)
}

/// Hard-links each image (same volume) or copies it.
fn link_folder(src: &Path, dest: &Path) -> Result<usize> {
    let mut files: Vec<String> = std::fs::read_dir(src)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| name_of(&e.path()))
        .filter(|n| is_page_image(n))
        .collect();
    files.sort_by(|a, b| natural_cmp(a, b));
    for (i, name) in files.iter().enumerate() {
        let target = dest.join(page_name(i, name));
        let from = src.join(name);
        if std::fs::hard_link(&from, &target).is_err() {
            std::fs::copy(&from, &target)?;
        }
    }
    Ok(files.len())
}

/// Extracts the images of a CBZ (nested folders included, macOS metadata skipped). Entry
/// names never become paths, so a crafted archive cannot write outside `dest`.
fn extract_cbz(src: &Path, dest: &Path) -> Result<usize> {
    let mut archive = zip::ZipArchive::new(File::open(src)?).map_err(zip_error)?;
    let mut pages: Vec<(usize, String)> = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index(i).ok()?;
            let name = file.name().ok()?.to_string();
            let wanted = file.is_file() && !name.starts_with("__MACOSX/") && is_page_image(&name);
            wanted.then_some((i, name))
        })
        .collect();
    pages.sort_by(|a, b| natural_cmp(&a.1, &b.1));
    for (n, (index, name)) in pages.iter().enumerate() {
        let mut file = archive.by_index(*index).map_err(zip_error)?;
        let mut out = File::create(dest.join(page_name(n, name)))?;
        io::copy(&mut file, &mut out)?;
    }
    Ok(pages.len())
}

fn zip_error(e: zip::result::ZipError) -> Error {
    Error::Other(format!("cbz: {e}"))
}
