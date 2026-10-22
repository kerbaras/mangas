//! PDF: one page per processed image, sized to the device screen. JPEGs are embedded as they
//! are (DCTDecode, no re-encoding); chapters become the document outline.

mod layout;
mod stream;

use std::io::{BufWriter, Cursor, Write};
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegDecoder;
use image::{ColorType, ImageDecoder};
use pdf_writer::writers::{Catalog, DocumentInfo};
use pdf_writer::{Content, Filter, Finish, Name, Rect, Ref, TextStr};

use super::epub::PageImage;
use super::output::write_file;
use super::{Exporter, Meta, PageSpool, Profile};
use crate::domain::{Error, Result};
use stream::{Objects, PdfStream};

use crate::domain::unplaced;
pub use layout::{page_size, place, Placement};

pub struct PdfExporter;

impl Exporter for PdfExporter {
    fn name(&self) -> &'static str {
        "PDF"
    }

    fn extension(&self) -> &'static str {
        "pdf"
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
            return Err(Error::Other("pdf: no pages".into()));
        }
        write_file(output, replace, |file| {
            write_document(&pages, meta, profile, BufWriter::new(file))
        })
    }
}

const CATALOG: Ref = Ref::new(1);
const TREE: Ref = Ref::new(2);
const INFO: Ref = Ref::new(3);
const OUTLINE: Ref = Ref::new(4);
const IMAGE: Name = Name(b"Im0");

/// Ids of page `i`: the page object, its image and its content stream.
fn page_refs(i: usize) -> (Ref, Ref, Ref) {
    let base = 10 + 3 * i as i32;
    (Ref::new(base), Ref::new(base + 1), Ref::new(base + 2))
}

/// Writes the document page by page, each page read from the spool just before it is embedded.
fn write_document(
    pages: &PageSpool,
    meta: &Meta,
    profile: &Profile,
    out: impl Write,
) -> Result<()> {
    let count = pages.len();
    let mut pdf = PdfStream::new(out)?;
    let mut head = Objects::new();
    head.at(INFO)
        .indirect(INFO)
        .start::<DocumentInfo>()
        .title(TextStr(&meta.title))
        .creator(TextStr("mangas"));
    let mut catalog = head.at(CATALOG).indirect(CATALOG).start::<Catalog>();
    catalog.pages(TREE);
    if !meta.chapters.is_empty() {
        catalog.outlines(OUTLINE);
    }
    catalog.finish();
    head.at(TREE)
        .pages(TREE)
        .kids((0..count).map(|i| page_refs(i).0))
        .count(count as i32);
    pdf.write(head)?;
    let size = page_size(profile);
    for i in 0..count {
        let page = pages.read(i)?;
        let mut objs = Objects::with_capacity(page.jpeg.len() + 4096);
        write_page(&mut objs, i, &page, size)?;
        pdf.write(objs)?;
    }
    let mut tail = Objects::new();
    write_outline(&mut tail, meta, count);
    pdf.write(tail)?;
    pdf.finish(CATALOG, INFO)?.flush().map_err(unplaced)
}

fn write_page(objs: &mut Objects, i: usize, page: &PageImage, size: (f32, f32)) -> Result<()> {
    let (page_id, image_id, content_id) = page_refs(i);
    let mut p = objs.at(page_id).page(page_id);
    p.parent(TREE)
        .media_box(Rect::new(0.0, 0.0, size.0, size.1))
        .contents(content_id);
    p.resources().x_objects().pair(IMAGE, image_id);
    p.finish();

    let mut image = objs.at(image_id).image_xobject(image_id, &page.jpeg);
    image.filter(Filter::DctDecode);
    image.width(page.width as i32).height(page.height as i32);
    image.bits_per_component(8);
    match jpeg_color(&page.jpeg)? {
        ColorType::L8 => image.color_space().device_gray(),
        _ => image.color_space().device_rgb(),
    };
    image.finish();

    let at = place((page.width, page.height), size);
    let mut content = Content::new();
    content.save_state();
    content.transform([at.width, 0.0, 0.0, at.height, at.x, at.y]);
    content.x_object(IMAGE);
    content.restore_state();
    objs.at(content_id).stream(content_id, &content.finish());
    Ok(())
}

/// Colour model from the JPEG header; the export only writes 8-bit grey or RGB.
fn jpeg_color(jpeg: &[u8]) -> Result<ColorType> {
    match JpegDecoder::new(Cursor::new(jpeg))?.color_type() {
        c @ (ColorType::L8 | ColorType::Rgb8) => Ok(c),
        other => Err(Error::Other(format!(
            "pdf: unsupported JPEG colour {other:?}"
        ))),
    }
}

/// One outline item per chapter, pointing at its first page.
fn write_outline(objs: &mut Objects, meta: &Meta, page_count: usize) {
    let chapters: Vec<_> = meta
        .chapters
        .iter()
        .filter(|c| c.first_page < page_count)
        .collect();
    if chapters.is_empty() {
        return;
    }
    let item = |j: usize| Ref::new(10 + 3 * page_count as i32 + j as i32);
    let last = chapters.len() - 1;
    objs.at(OUTLINE)
        .outline(OUTLINE)
        .first(item(0))
        .last(item(last))
        .count(chapters.len() as i32);
    for (j, chapter) in chapters.iter().enumerate() {
        let mut entry = objs.at(item(j)).outline_item(item(j));
        entry.parent(OUTLINE).title(TextStr(&chapter.title));
        if j > 0 {
            entry.prev(item(j - 1));
        }
        if j < last {
            entry.next(item(j + 1));
        }
        entry.dest().page(page_refs(chapter.first_page).0).fit();
    }
}
