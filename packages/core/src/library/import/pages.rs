//! Writes the pages of one entry into a chapter directory as `0001.<ext>`, `0002.<ext>`, ...
//! in natural order, so the exporter's name sort reads them in reading order.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use super::names::{extension, is_page_image, natural_cmp};
use super::scan::{name_of, Entry, EntryKind};
use crate::domain::{Error, Result};

/// Replaces `dest` with the pages of `entry`; returns how many were written.
pub fn write_pages(entry: &Entry, dest: &Path) -> Result<usize> {
    if dest.exists() {
        std::fs::remove_dir_all(dest)?;
    }
    std::fs::create_dir_all(dest).map_err(|e| not_writable(dest, &e))?;
    match entry.kind {
        EntryKind::Folder => copy_folder(&entry.path, dest),
        EntryKind::Cbz => extract_cbz(&entry.path, dest),
    }
}

fn page_name(index: usize, original: &str) -> String {
    let ext = extension(original).unwrap_or_else(|| "jpg".into());
    format!("{:04}.{ext}", index + 1)
}

/// Copies each image, so later edits to the source folder never reach the library.
fn copy_folder(src: &Path, dest: &Path) -> Result<usize> {
    let mut files: Vec<String> = std::fs::read_dir(src)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| name_of(&e.path()))
        .filter(|n| is_page_image(n))
        .collect();
    files.sort_by(|a, b| natural_cmp(a, b));
    for (i, name) in files.iter().enumerate() {
        std::fs::copy(src.join(name), dest.join(page_name(i, name)))?;
    }
    Ok(files.len())
}

/// Extracts the images of a CBZ (nested folders included, macOS metadata skipped). Entry
/// names never become paths, so a crafted archive cannot write outside `dest`.
fn extract_cbz(src: &Path, dest: &Path) -> Result<usize> {
    let mut archive = zip::ZipArchive::new(File::open(src)?).map_err(|e| match e {
        // A truncated archive also surfaces as an IO error; only real read failures stay IO.
        zip::result::ZipError::Io(e)
            if !matches!(
                e.kind(),
                io::ErrorKind::UnexpectedEof | io::ErrorKind::InvalidData
            ) =>
        {
            Error::Io(e)
        }
        _ => Error::Import(format!("{} isn't a valid CBZ file", name_of(src))),
    })?;
    let mut pages: Vec<(usize, String)> = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index(i).ok()?;
            let name = file.name().ok()?.to_string();
            let wanted = file.is_file() && !name.starts_with("__MACOSX/") && is_page_image(&name);
            wanted.then_some((i, name))
        })
        .collect();
    pages.sort_by(|a, b| natural_cmp(&a.1, &b.1));
    let mut total = 0;
    for (n, (index, name)) in pages.iter().enumerate() {
        let mut file = archive.by_index(*index).map_err(zip_error)?;
        let mut out = File::create(dest.join(page_name(n, name)))?;
        let budget = MAX_PAGE_BYTES.min(MAX_CHAPTER_BYTES - total);
        let written = io::copy(&mut (&mut file).take(budget + 1), &mut out)?;
        if written > budget {
            return Err(Error::Other(format!(
                "cbz: {name} goes over the size limit"
            )));
        }
        total += written;
    }
    Ok(pages.len())
}

/// No real page or chapter comes close; stops a crafted archive from filling the disk.
const MAX_PAGE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CHAPTER_BYTES: u64 = 4 * 1024 * 1024 * 1024;

fn zip_error(e: zip::result::ZipError) -> Error {
    Error::Other(format!("cbz: {e}"))
}

fn not_writable(dest: &Path, e: &io::Error) -> Error {
    Error::Import(format!(
        "Couldn't write to {}: {}",
        dest.display(),
        e.kind()
    ))
}
