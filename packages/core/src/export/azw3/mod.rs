//! Fixed-layout AZW3 (KF8) writer: the book's page JPEGs in a KF8-only MOBI file a Kindle reads
//! side-loaded over USB. Pure apart from reading pages from the spool and writing the output.
//!
//! Written from the format descriptions (MobileRead wiki "PDB", "MOBI" and "KF8", Amazon's
//! Kindle Publishing Guidelines); calibre was only read to check field meanings. Format
//! decisions, where the sources leave a choice:
//! - KF8 only, no MOBI 7 part (file version 8): every Kindle since the Paperwhite 1 reads it.
//! - Uncompressed text, extra data flags = 1 (multibyte trailing byte only, no TBS): the text
//!   is a few hundred bytes per page; the trailing byte is what kindlegen files always carry.
//!   Trailing byte sequences only steer MOBI 6 navigation (MobileRead "MOBI", TBS section), so
//!   `calibre-debug --inspect-mobi` prints "TBS mismatch" for each text record of this file.
//! - MOBI header field 0x28 (orth index) is NULL: a value there marks a dictionary.
//! - Each page keeps its JPEG unchanged in a `CRES` record; its standard record is the same
//!   JPEG when it fits 128 KiB, else a smaller copy ([`images`]).
//! - The cover, when the book has one, is the first page as in the EPUB; EXTH 201 points at
//!   it, or at the first page.
//! - RTL books keep the pages in reading order; EXTH 527 `rtl` makes pages turn right to left,
//!   as in the EPUB's spine.
//! - EXTH 501 `EBOK` with EXTH 113 = the book id, so a side-loaded book is filed as a book.

mod bytes;
mod cncx;
mod exth;
mod fdst;
mod hd;
mod header0;
mod images;
mod indx;
mod layout;
mod palmdb;
mod tables;
mod text;
mod writer;

pub use images::copy_bound;
pub use writer::{write, Azw3Exporter, Report};
