//! Writes the pages of one entry into a chapter directory as `0001.<ext>`, `0002.<ext>`, ...
//! in natural order, so the exporter's name sort reads them in reading order. Every page goes
//! through the shared image check first; one that fails is left out with the reason.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use super::names::{is_page_image, natural_cmp};
use super::report::Skipped;
use super::scan::{name_of, Entry, EntryKind};
use crate::domain::{io_at, IoAt};
use crate::domain::{Error, Result};
use crate::images::check_page;

/// What [`write_pages`] wrote, and the pages it left out (named inside the entry).
#[derive(Debug, Default)]
pub struct Written {
    pub pages: usize,
    pub skipped: Vec<Skipped>,
}

impl Written {
    /// Checks `bytes` and stores them as the next page, named for their real format.
    fn add(&mut self, dest: &Path, name: &str, bytes: &[u8]) -> Result<()> {
        match check_page(bytes) {
            Ok(ext) => {
                self.pages += 1;
                let file = dest.join(format!("{:04}.{ext}", self.pages));
                std::fs::write(&file, bytes).at(&file)?;
            }
            Err(reason) => self.skipped.push(Skipped::new(name, &reason)),
        }
        Ok(())
    }
}

/// Replaces `dest` with the valid pages of `entry`.
pub fn write_pages(entry: &Entry, dest: &Path) -> Result<Written> {
    if dest.exists() {
        std::fs::remove_dir_all(dest).at(dest)?;
    }
    std::fs::create_dir_all(dest).map_err(|e| not_writable(dest, &e))?;
    match entry.kind {
        EntryKind::Folder => copy_folder(&entry.path, dest),
        EntryKind::Cbz => extract_cbz(&entry.path, dest),
    }
}

/// Copies each image, so later edits to the source folder never reach the library.
fn copy_folder(src: &Path, dest: &Path) -> Result<Written> {
    let mut files: Vec<String> = std::fs::read_dir(src)
        .at(src)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| name_of(&e.path()))
        .filter(|n| is_page_image(n))
        .collect();
    files.sort_by(|a, b| natural_cmp(a, b));
    let mut written = Written::default();
    for name in &files {
        let page = src.join(name);
        written.add(dest, name, &std::fs::read(&page).at(&page)?)?;
    }
    Ok(written)
}

/// Extracts the images of a CBZ (nested folders included, macOS metadata skipped). Entry
/// names never become paths, so a crafted archive cannot write outside `dest`.
fn extract_cbz(src: &Path, dest: &Path) -> Result<Written> {
    let mut archive = zip::ZipArchive::new(File::open(src).at(src)?).map_err(|e| match e {
        // A truncated archive also surfaces as an IO error; only real read failures stay IO.
        zip::result::ZipError::Io(e)
            if !matches!(
                e.kind(),
                io::ErrorKind::UnexpectedEof | io::ErrorKind::InvalidData
            ) =>
        {
            io_at(src)(e)
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
    let mut written = Written::default();
    for (index, name) in &pages {
        let file = archive.by_index(*index).map_err(zip_error)?;
        let budget = MAX_PAGE_BYTES.min(MAX_CHAPTER_BYTES - total);
        let mut bytes = Vec::new();
        file.take(budget + 1).read_to_end(&mut bytes).at(src)?;
        if bytes.len() as u64 > budget {
            return Err(Error::Other(format!(
                "cbz: {name} goes over the size limit"
            )));
        }
        total += bytes.len() as u64;
        written.add(dest, name, &bytes)?;
    }
    Ok(written)
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
