//! Fixed-layout EPUB3 writer for comics: `mimetype` stored first, one XHTML per page with a
//! full-bleed image, pre-paginated rendition metadata that Kindle Previewer / Send to Kindle
//! accept as a comic.

mod exporter;
mod modified;
mod nav;
mod xml;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use super::output::write_file;
use super::PageSpool;
use crate::domain::{Error, Result};

pub use exporter::EpubExporter;

/// A page image already encoded as JPEG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageImage {
    pub jpeg: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// A chapter is a title and the index of its first page in `Book::pages`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterEntry {
    pub title: String,
    pub first_page: usize,
}

/// The series a book belongs to and its place in it (the volume number), when known.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Series {
    pub name: String,
    pub position: Option<String>,
}

#[derive(Debug)]
pub struct Book {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub language: String,
    pub series: Option<Series>,
    pub right_to_left: bool,
    pub device_width: u32,
    pub device_height: u32,
    pub chapters: Vec<ChapterEntry>,
    /// Read back one at a time while the zip is written.
    pub pages: PageSpool,
    /// A separate cover page shown first; without one the first page is the cover image.
    pub cover: Option<PageImage>,
}

pub(crate) const COVER_XHTML: &str = "cover.xhtml";
pub(crate) const COVER_IMAGE: &str = "cover.jpg";

pub(crate) struct PageEntry {
    pub index: usize,
    pub image_name: String,
    pub xhtml_name: String,
    pub width: u32,
    pub height: u32,
}

fn entries(book: &Book) -> Vec<PageEntry> {
    book.pages
        .info()
        .iter()
        .enumerate()
        .map(|(i, p)| PageEntry {
            index: i,
            image_name: format!("page-{i:05}.jpg"),
            xhtml_name: format!("page-{i:05}.xhtml"),
            width: p.width,
            height: p.height,
        })
        .collect()
}

/// Writes `book` at `path` and returns the file written: over an existing file when `replace`,
/// else under a free ` (n)` variant; see [`write_file`].
pub fn write(book: &Book, path: &Path, replace: bool) -> Result<PathBuf> {
    if book.pages.is_empty() {
        return Err(Error::Other("epub: no pages".into()));
    }
    write_file(path, replace, |file| write_zip(book, file))
}

fn write_zip(book: &Book, file: File) -> Result<()> {
    crate::zip_out::write_zip(BufWriter::new(file), "epub", |zip| {
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let z = crate::export::space::zip_error("epub");

        zip.start_file("mimetype", stored).map_err(z)?;
        zip.write_all(b"application/epub+zip")?;
        zip.start_file("META-INF/container.xml", deflated)
            .map_err(z)?;
        zip.write_all(xml::CONTAINER_XML.as_bytes())?;

        let pages = entries(book);
        zip.start_file("OEBPS/content.opf", deflated).map_err(z)?;
        zip.write_all(xml::content_opf(book, &pages, &modified::modified(book)?).as_bytes())?;
        zip.start_file("OEBPS/nav.xhtml", deflated).map_err(z)?;
        zip.write_all(nav::nav_xhtml(book, &pages).as_bytes())?;
        if let Some(cover) = &book.cover {
            zip.start_file(format!("OEBPS/text/{COVER_XHTML}"), deflated)
                .map_err(z)?;
            zip.write_all(xml::cover_xhtml(&book.title, cover).as_bytes())?;
            zip.start_file(format!("OEBPS/images/{COVER_IMAGE}"), stored)
                .map_err(z)?;
            zip.write_all(&cover.jpeg)?;
        }
        for entry in &pages {
            let page = book.pages.read(entry.index)?;
            zip.start_file(format!("OEBPS/text/{}", entry.xhtml_name), deflated)
                .map_err(z)?;
            zip.write_all(xml::page_xhtml(&book.title, entry).as_bytes())?;
            zip.start_file(format!("OEBPS/images/{}", entry.image_name), stored)
                .map_err(z)?;
            zip.write_all(&page.jpeg)?;
        }
        Ok(())
    })
}
