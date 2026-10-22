//! The EXTH metadata block of record 0. Record numbers from the MobileRead wiki "MOBI" page and
//! Amazon's Kindle Publishing Guidelines (fixed-layout metadata): 122 fixed-layout, 123 book
//! type, 126 original resolution, 127/128 zero gutter/margin, 524 language, 525 primary writing
//! mode, 527 page progression direction, 201/202 cover/thumbnail (image index from the first
//! image record), 129 the KF8 cover URI, 536 the HD container geometry.

use super::bytes::{pad4, push32, u32_of};
use crate::domain::Result;

/// What the EXTH block says about a book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookExth<'a> {
    pub title: &'a str,
    pub authors: &'a [String],
    pub language: &'a str,
    pub right_to_left: bool,
    /// The device size, `original-resolution`.
    pub resolution: (u32, u32),
    /// Asin-like id so side-loaded books are filed as books with their cover (see `asin`).
    pub asin: String,
    pub cover: u32,
    pub thumbnail: u32,
    /// Image records in the book.
    pub resources: u32,
    /// EXTH 536 when the book carries an HD container.
    pub hd_geometry: Option<String>,
}

/// One EXTH record: type, length including this 8-byte head, data.
pub fn record(kind: u32, data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(8 + data.len());
    push32(&mut out, kind);
    push32(&mut out, u32_of(8 + data.len())?);
    out.extend_from_slice(data);
    Ok(out)
}

/// `EXTH`, the block length (without padding), the record count, the records, zero padding to
/// four bytes.
pub fn block(records: &[(u32, Vec<u8>)]) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    for (kind, data) in records {
        body.extend(record(*kind, data)?);
    }
    let mut out = b"EXTH".to_vec();
    push32(&mut out, u32_of(12 + body.len())?);
    push32(&mut out, u32_of(records.len())?);
    out.extend(body);
    pad4(&mut out);
    Ok(out)
}

/// The records of a fixed-layout comic, in a fixed order.
pub fn book_records(b: &BookExth) -> Vec<(u32, Vec<u8>)> {
    let text = |kind: u32, value: &str| (kind, value.as_bytes().to_vec());
    let number = |kind: u32, value: u32| (kind, value.to_be_bytes().to_vec());
    let (mode, progression) = match b.right_to_left {
        true => ("horizontal-rl", "rtl"),
        false => ("horizontal-lr", "ltr"),
    };
    let mut records: Vec<_> = b.authors.iter().map(|a| text(100, a)).collect();
    records.extend([
        text(503, b.title),
        text(524, b.language),
        text(525, mode),
        text(527, progression),
        text(122, "true"),
        text(123, "comic"),
        text(124, "none"),
        text(126, &format!("{}x{}", b.resolution.0, b.resolution.1)),
        text(127, "true"),
        text(128, "true"),
        text(501, "EBOK"),
        text(113, &b.asin),
        number(201, b.cover),
        number(202, b.thumbnail),
        number(203, 0),
        text(
            129,
            &format!(
                "kindle:embed:{}",
                super::bytes::base32(b.cover as usize + 1, 4)
            ),
        ),
        number(125, b.resources),
        number(131, 0),
        text(535, "mangas"),
    ]);
    records.extend(b.hd_geometry.as_deref().map(|g| text(536, g)));
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(rtl: bool) -> BookExth<'static> {
        BookExth {
            title: "Yotsuba&!",
            authors: &[],
            language: "en",
            right_to_left: rtl,
            resolution: (1264, 1680),
            asin: "abc".into(),
            cover: 0,
            thumbnail: 3,
            resources: 4,
            hd_geometry: None,
        }
    }

    fn get(records: &[(u32, Vec<u8>)], kind: u32) -> Vec<u8> {
        records
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, v)| v.clone())
            .unwrap()
    }

    #[test]
    fn fixed_layout_records_follow_the_direction() {
        let ltr = book_records(&book(false));
        assert_eq!(get(&ltr, 122), b"true");
        assert_eq!(get(&ltr, 123), b"comic");
        assert_eq!(get(&ltr, 126), b"1264x1680");
        assert_eq!(get(&ltr, 525), b"horizontal-lr");
        assert_eq!(get(&ltr, 527), b"ltr");
        assert_eq!(get(&ltr, 202), 3u32.to_be_bytes());
        assert_eq!(get(&ltr, 129), b"kindle:embed:0001");
        let rtl = book_records(&book(true));
        assert_eq!(get(&rtl, 525), b"horizontal-rl");
        assert_eq!(get(&rtl, 527), b"rtl");
    }

    #[test]
    fn block_is_padded_and_counted() {
        let b = block(&[(100, b"abc".to_vec())]).unwrap();
        assert_eq!(&b[..4], b"EXTH");
        assert_eq!(u32::from_be_bytes([b[4], b[5], b[6], b[7]]), 23);
        assert_eq!(u32::from_be_bytes([b[8], b[9], b[10], b[11]]), 1);
        assert_eq!(b.len(), 24);
    }
}
