//! Record 0 of a KF8-only book: the PalmDOC header, the 264-byte MOBI header of file version 8,
//! the EXTH block and the full title (MobileRead wiki "MOBI"; KF8 fields from the "KF8" page).
//! Record numbers count from this record, which is also the first record of the file.

use super::bytes::{push16, push32, put32, u16_of, u32_of, NULL};
use super::text::RECORD_SIZE;
use crate::domain::Result;

const MOBI_LEN: usize = 264;
/// kindlegen pads record 0 to at least this size; some readers expect the room.
const MIN_LEN: usize = 8892;

/// Where the parts of the book are, by record number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header0<'a> {
    pub title: &'a str,
    pub language: &'a str,
    pub uid: u32,
    pub text_len: usize,
    pub text_records: usize,
    pub first_non_book: usize,
    pub first_image: usize,
    pub fragment: usize,
    pub skeleton: usize,
    pub ncx: usize,
    pub guide: usize,
    pub fdst: usize,
    pub flis: usize,
    pub fcis: usize,
    pub exth: Vec<u8>,
}

pub fn build(h: &Header0) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(MIN_LEN);
    push16(&mut out, 1); // no compression
    push16(&mut out, 0);
    push32(&mut out, u32_of(h.text_len)?);
    push16(&mut out, u16_of(h.text_records)?);
    push16(&mut out, RECORD_SIZE as u16);
    push32(&mut out, 0); // no encryption
    let mut mobi = vec![0; MOBI_LEN];
    mobi[..4].copy_from_slice(b"MOBI");
    let r = |n: usize| u32_of(n);
    let fields = [
        (4, MOBI_LEN as u32),
        (8, 2), // book
        (12, 65001),
        (16, h.uid),
        (20, 8),
        (64, r(h.first_non_book)?),
        (68, r(16 + MOBI_LEN + h.exth.len())?),
        (72, r(h.title.len())?),
        (76, locale(h.language)),
        (88, 8),
        (92, r(h.first_image)?),
        (112, 0x50), // has EXTH
        (176, r(h.fdst)?),
        (180, 1),
        (184, r(h.fcis)?),
        (188, 1),
        (192, r(h.flis)?),
        (196, 1),
        (224, 1), // extra data: multibyte trailing entry only
        (228, r(h.ncx)?),
        (232, r(h.fragment)?),
        (236, r(h.skeleton)?),
        (244, r(h.guide)?),
    ];
    // 24 is the dictionary orth index: NULL, or the firmware treats the book as a dictionary
    // (calibre's KF8 writer and KindleUnpack's KF8 header both read it as `metaorthindex`).
    for at in (24..64)
        .step_by(4)
        .chain([148, 152, 208, 216, 220, 240, 248, 256])
    {
        put32(&mut mobi, at, NULL);
    }
    for (at, value) in fields {
        put32(&mut mobi, at, value);
    }
    out.extend(mobi);
    out.extend_from_slice(&h.exth);
    out.extend_from_slice(h.title.as_bytes());
    out.resize((out.len() + 2).div_ceil(4) * 4, 0);
    out.resize(out.len().max(MIN_LEN), 0);
    Ok(out)
}

/// The MOBI locale field: the Windows primary language id of the book's language (the EXTH
/// 524 string carries the exact tag; this field only steers the dictionary lookup).
fn locale(language: &str) -> u32 {
    let primary = language
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match primary.as_str() {
        "ar" => 0x01,
        "zh" => 0x04,
        "de" => 0x07,
        "en" => 0x09,
        "es" => 0x0A,
        "fr" => 0x0C,
        "it" => 0x10,
        "ja" => 0x11,
        "ko" => 0x12,
        "nl" => 0x13,
        "pl" => 0x15,
        "pt" => 0x16,
        "ru" => 0x19,
        "id" => 0x21,
        "vi" => 0x2A,
        "th" => 0x1E,
        "tr" => 0x1F,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn be32(b: &[u8], at: usize) -> u32 {
        u32::from_be_bytes(b[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn kf8_fields_land_at_their_offsets() {
        let h = Header0 {
            title: "Yotsuba",
            language: "en-US",
            uid: 7,
            text_len: 5000,
            text_records: 2,
            first_non_book: 3,
            first_image: 4,
            fragment: 10,
            skeleton: 13,
            ncx: 15,
            guide: 18,
            fdst: 21,
            flis: 22,
            fcis: 23,
            exth: b"EXTH\0\0\0\x0c\0\0\0\0".to_vec(),
        };
        let r = build(&h).unwrap();
        assert_eq!(r.len(), MIN_LEN);
        assert_eq!(&r[16..20], b"MOBI");
        assert_eq!(be32(&r, 16 + 20), 8);
        assert_eq!(be32(&r, 16 + 76), 9);
        assert_eq!(be32(&r, 16 + 176), 21);
        assert_eq!(be32(&r, 16 + 192), 22);
        assert_eq!(be32(&r, 16 + 236), 13);
        assert_eq!(&r[16 + 264..16 + 268], b"EXTH");
        let at = be32(&r, 16 + 68) as usize;
        assert_eq!(&r[at..at + 7], b"Yotsuba");
        assert_eq!(r[at + 7], 0);
    }
}
