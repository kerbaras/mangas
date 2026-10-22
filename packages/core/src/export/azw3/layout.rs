//! The book as an ordered list of PalmDB records. Pure: the HD images are named by where they
//! come from, so the writer can stream them from the spool.
//!
//! Order (kindlegen's for a KF8-only book): record 0, the text records, a 2-byte pad record,
//! the standard image records (cover, pages, thumbnail), the FRAG, SKEL, NCX and guide indexes
//! (each with its CNCX), FDST, FLIS, FCIS, EOF, then the HD container.

use super::exth::{self, BookExth};
use super::fdst::{fcis, fdst, flis, EOF};
use super::hd::{self, Container};
use super::header0::{self, Header0};
use super::tables::{self, TocEntry};
use super::text::{self, Page};
use crate::domain::Result;

/// Where an HD image's bytes come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Cover,
    Page(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    Bytes(Vec<u8>),
    /// A `CRES` record: its head, then the `len` bytes of `source`, unchanged.
    Cres {
        source: Source,
        len: usize,
    },
}

impl Record {
    pub fn len(&self) -> usize {
        match self {
            Record::Bytes(b) => b.len(),
            Record::Cres { len, .. } => hd::CRES_HEAD + len,
        }
    }
}

/// A page image: its standard record and its original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub standard: Vec<u8>,
    pub source: Source,
    pub original_len: usize,
    pub width: u32,
    pub height: u32,
    pub label: String,
}

/// Everything the book's records are built from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Book<'a> {
    pub title: &'a str,
    pub authors: &'a [String],
    pub language: &'a str,
    pub id: &'a str,
    pub right_to_left: bool,
    pub resolution: (u32, u32),
    /// In reading order, the cover first when there is one.
    pub images: Vec<Image>,
    pub thumbnail: Vec<u8>,
    pub toc: Vec<TocEntry>,
}

pub fn records(book: &Book) -> Result<Vec<Record>> {
    let pages: Vec<Page> = (book.images.iter().enumerate())
        .map(|(i, img)| Page {
            image: i + 1,
            width: img.width,
            height: img.height,
            label: img.label.clone(),
        })
        .collect();
    let text = text::build(book.title, &pages);
    let text_records = text::records(&text.bytes);
    let first_image = text_records.len() + 2;
    let image_count = book.images.len() + 1;
    let fragment = tables::fragment(&text.files)?;
    let skeleton = tables::skeleton(&text.files)?;
    let ncx = tables::ncx(&text.files, text.bytes.len(), &book.toc)?;
    let guide = tables::guide()?;
    let at_fragment = first_image + image_count;
    let at_skeleton = at_fragment + fragment.len();
    let at_ncx = at_skeleton + skeleton.len();
    let at_guide = at_ncx + ncx.len();
    let at_fdst = at_guide + guide.len();
    let container = Container {
        slots: (0..image_count).map(|i| i < book.images.len()).collect(),
        max: (book.images.iter()).fold((0, 0), |(w, h), img| (w.max(img.width), h.max(img.height))),
    };
    let meta = BookExth {
        title: book.title,
        authors: book.authors,
        language: book.language,
        right_to_left: book.right_to_left,
        resolution: book.resolution,
        asin: book.id.to_string(),
        cover: 0,
        thumbnail: super::bytes::u32_of(book.images.len())?,
        resources: super::bytes::u32_of(image_count)?,
        hd_geometry: Some(container.geometry()),
    };
    let record0 = header0::build(&Header0 {
        title: book.title,
        language: book.language,
        uid: uid(book.id),
        text_len: text.bytes.len(),
        text_records: text_records.len(),
        first_non_book: text_records.len() + 1,
        first_image,
        fragment: at_fragment,
        skeleton: at_skeleton,
        ncx: at_ncx,
        guide: at_guide,
        fdst: at_fdst,
        flis: at_fdst + 1,
        fcis: at_fdst + 2,
        exth: exth::block(&exth::book_records(&meta))?,
    })?;
    let text_len = super::bytes::u32_of(text.bytes.len())?;
    let mut out: Vec<Record> = vec![Record::Bytes(record0)];
    let owned = text_records.into_iter().chain([vec![0, 0]]);
    let owned = owned.chain(book.images.iter().map(|i| i.standard.clone()));
    let owned = owned.chain([book.thumbnail.clone()]);
    let owned = owned
        .chain(fragment)
        .chain(skeleton)
        .chain(ncx)
        .chain(guide);
    let owned = owned.chain([fdst(text_len), flis(), fcis(text_len), EOF.to_vec()]);
    let owned = owned.chain([hd::BOUNDARY.to_vec(), container.header(book.title)?]);
    out.extend(owned.map(Record::Bytes));
    out.extend(book.images.iter().map(|i| Record::Cres {
        source: i.source,
        len: i.original_len,
    }));
    out.push(Record::Bytes(hd::PLACEHOLDER.to_vec()));
    let tail = [
        container.embed_list(),
        hd::CONT_BOUNDARY.to_vec(),
        EOF.to_vec(),
    ];
    out.extend(tail.map(Record::Bytes));
    Ok(out)
}

/// The MOBI unique id: the first four bytes of the book id's SHA-256, stable across runs.
fn uid(id: &str) -> u32 {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(id.as_bytes());
    u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]])
}
