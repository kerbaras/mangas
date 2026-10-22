//! The IO edge: standard image copies from the spool, the records from [`layout`], streamed to
//! the output with each HD original read back from the spool as it is written.

use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use super::images;
use super::layout::{self, Book, Image, Record, Source};
use super::tables::TocEntry;
use crate::domain::{io_at, Result};
use crate::export::epub::PageImage;
use crate::export::{write_file, Exporter, Meta, PageSpool, Profile};

/// What the export did beyond the pages: which ones needed a smaller standard copy (indexes
/// into the pages, reading order) and whether the cover did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub smaller_pages: Vec<usize>,
    pub smaller_cover: bool,
}

pub struct Azw3Exporter;

impl Exporter for Azw3Exporter {
    fn name(&self) -> &'static str {
        "AZW3"
    }

    fn extension(&self) -> &'static str {
        "azw3"
    }

    fn export(
        &self,
        pages: PageSpool,
        meta: &Meta,
        profile: &Profile,
        output: &Path,
        replace: bool,
    ) -> Result<PathBuf> {
        write_file(output, replace, |file| {
            let mut out = BufWriter::new(file);
            let report = write(&pages, meta, profile, &mut out)?;
            out.flush().map_err(io_at(output))?;
            tracing::info!(pages = ?report.smaller_pages, cover = report.smaller_cover, "azw3: smaller standard image copies");
            Ok(())
        })
    }
}

/// Writes the book to `out` and reports which pages got a smaller standard copy.
pub fn write(
    pages: &PageSpool,
    meta: &Meta,
    profile: &Profile,
    out: &mut impl Write,
) -> Result<Report> {
    let standard: Vec<Option<Vec<u8>>> = (0..pages.len())
        .into_par_iter()
        .map(|i| images::low_res(&pages.read(i)?.jpeg))
        .collect::<Result<_>>()?;
    let cover = meta
        .cover
        .as_ref()
        .map(|c| Ok::<_, crate::domain::Error>((c, images::low_res(&c.jpeg)?)))
        .transpose()?;
    let report = Report {
        smaller_pages: (standard.iter().enumerate())
            .filter(|(_, s)| s.is_some())
            .map(|(i, _)| i)
            .collect(),
        smaller_cover: cover.as_ref().is_some_and(|(_, s)| s.is_some()),
    };
    let mut list = Vec::with_capacity(pages.len() + 1);
    if let Some((c, small)) = cover {
        list.push(image(
            c.jpeg.len(),
            c.width,
            c.height,
            Source::Cover,
            small,
            || Ok(c.jpeg.clone()),
            "Cover",
        )?);
    }
    for (i, (info, small)) in pages.info().iter().zip(standard).enumerate() {
        let label = format!("Page {}", i + 1);
        list.push(image(
            info.bytes,
            info.width,
            info.height,
            Source::Page(i),
            small,
            || Ok(pages.read(i)?.jpeg),
            &label,
        )?);
    }
    let thumb_from = match &meta.cover {
        Some(c) => c.jpeg.clone(),
        None => pages.read(0)?.jpeg,
    };
    let offset = usize::from(meta.cover.is_some());
    let mut toc: Vec<TocEntry> = (meta.chapters.iter())
        .map(|c| TocEntry {
            label: c.title.clone(),
            file: c.first_page + offset,
        })
        .collect();
    if toc.is_empty() {
        toc.push(TocEntry {
            label: meta.title.clone(),
            file: 0,
        });
    }
    let book = Book {
        title: &meta.title,
        authors: &meta.authors,
        language: &meta.language,
        id: &meta.id,
        right_to_left: meta.right_to_left,
        resolution: (profile.width, profile.height),
        images: list,
        thumbnail: images::thumbnail(&thumb_from)?,
        toc,
    };
    let records = layout::records(&book)?;
    stream(&records, book.title, pages, meta.cover.as_ref(), out)?;
    Ok(report)
}

fn image(
    len: usize,
    width: u32,
    height: u32,
    source: Source,
    small: Option<Vec<u8>>,
    original: impl FnOnce() -> Result<Vec<u8>>,
    label: &str,
) -> Result<Image> {
    let standard = match small {
        Some(s) => s,
        None => original()?,
    };
    Ok(Image {
        standard,
        source,
        original_len: len,
        width,
        height,
        label: label.into(),
    })
}

fn stream(
    records: &[Record],
    title: &str,
    pages: &PageSpool,
    cover: Option<&PageImage>,
    out: &mut impl Write,
) -> Result<()> {
    let at = Path::new("azw3 output");
    let sizes: Vec<usize> = records.iter().map(Record::len).collect();
    out.write_all(&super::palmdb::header(title, &sizes)?)
        .map_err(io_at(at))?;
    for record in records {
        match record {
            Record::Bytes(b) => out.write_all(b).map_err(io_at(at))?,
            Record::Cres { source, .. } => {
                out.write_all(&super::hd::cres_head()).map_err(io_at(at))?;
                let jpeg = match (source, cover) {
                    (Source::Cover, Some(c)) => c.jpeg.clone(),
                    (Source::Page(i), _) => pages.read(*i)?.jpeg,
                    (Source::Cover, None) => Vec::new(),
                };
                out.write_all(&jpeg).map_err(io_at(at))?;
            }
        }
    }
    Ok(())
}
