//! Fixed-layout EPUB3 writer for comics: `mimetype` stored first, one XHTML per page with a
//! full-bleed image, pre-paginated rendition metadata that Kindle Previewer / Send to Kindle
//! accept as a comic.

mod xml;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::domain::{Error, Result};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Book {
    pub id: String,
    pub title: String,
    pub author: String,
    pub language: String,
    pub right_to_left: bool,
    pub device_width: u32,
    pub device_height: u32,
    pub chapters: Vec<ChapterEntry>,
    pub pages: Vec<PageImage>,
}

pub(crate) struct PageEntry {
    pub index: usize,
    pub image_name: String,
    pub xhtml_name: String,
    pub width: u32,
    pub height: u32,
}

fn entries(book: &Book) -> Vec<PageEntry> {
    book.pages
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

/// Writes `book` to `path` (parent directories are created).
pub fn write(book: &Book, path: &Path) -> Result<()> {
    if book.pages.is_empty() {
        return Err(Error::Other("epub: no pages".into()));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = BufWriter::new(File::create(path)?);
    let mut zip = ZipWriter::new(file);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let z = |e: zip::result::ZipError| Error::Other(format!("epub: {e}"));

    zip.start_file("mimetype", stored).map_err(z)?;
    zip.write_all(b"application/epub+zip")?;
    zip.start_file("META-INF/container.xml", deflated)
        .map_err(z)?;
    zip.write_all(xml::CONTAINER_XML.as_bytes())?;

    let pages = entries(book);
    let modified = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    zip.start_file("OEBPS/content.opf", deflated).map_err(z)?;
    zip.write_all(xml::content_opf(book, &pages, &modified).as_bytes())?;
    zip.start_file("OEBPS/nav.xhtml", deflated).map_err(z)?;
    zip.write_all(xml::nav_xhtml(book, &pages).as_bytes())?;
    for (entry, page) in pages.iter().zip(&book.pages) {
        zip.start_file(format!("OEBPS/text/{}", entry.xhtml_name), deflated)
            .map_err(z)?;
        zip.write_all(xml::page_xhtml(&book.title, entry).as_bytes())?;
        zip.start_file(format!("OEBPS/images/{}", entry.image_name), stored)
            .map_err(z)?;
        zip.write_all(&page.jpeg)?;
    }
    zip.finish().map_err(z)?.flush()?;
    Ok(())
}
