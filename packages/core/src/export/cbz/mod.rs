//! CBZ: the processed JPEG pages in a zip, in reading order, plus a `ComicInfo.xml` that
//! comic readers (Kobo, Komga, Kavita, tablet apps) use for title, page list and chapters.

mod comic_info;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use super::output::write_file;
use super::{Exporter, Meta, PageSpool, Profile};
use crate::domain::{Error, Result};

use crate::domain::unplaced;
pub use comic_info::comic_info_xml;

pub struct CbzExporter;

impl Exporter for CbzExporter {
    fn name(&self) -> &'static str {
        "CBZ"
    }

    fn extension(&self) -> &'static str {
        "cbz"
    }

    fn export(
        &self,
        pages: PageSpool,
        meta: &Meta,
        profile: &Profile,
        output: &Path,
        replace: bool,
    ) -> Result<PathBuf> {
        if pages.is_empty() {
            return Err(Error::Other("cbz: no pages".into()));
        }
        write_file(output, replace, |file| {
            write_zip(&pages, meta, profile, file)
        })
    }
}

/// Name of page `index` (0-based): zero-padded so every reader sorts them in order.
pub fn page_name(index: usize, total: usize) -> String {
    let width = total.to_string().len().max(4);
    format!("{:0width$}.jpg", index + 1)
}

fn write_zip(pages: &PageSpool, meta: &Meta, profile: &Profile, file: File) -> Result<()> {
    crate::zip_out::write_zip(BufWriter::new(file), "cbz", |zip| {
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let z = crate::export::space::zip_error("cbz");
        for i in 0..pages.len() {
            let page = pages.read(i)?;
            zip.start_file(page_name(i, pages.len()), stored)
                .map_err(z)?;
            zip.write_all(&page.jpeg).map_err(unplaced)?;
        }
        zip.start_file("ComicInfo.xml", deflated).map_err(z)?;
        zip.write_all(comic_info_xml(pages.info(), meta, profile).as_bytes())
            .map_err(unplaced)?;
        Ok(())
    })
}
