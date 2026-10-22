//! The PalmDB container: a 78-byte header, one 8-byte directory entry per record and two bytes
//! of gap before the first record (Palm Database Format, MobileRead wiki "PDB").

use super::bytes::{push16, push32, u16_of, u32_of};
use crate::domain::Result;

const HEADER_LEN: usize = 78;
const NAME_LEN: usize = 32;

/// The header and record directory for records of `sizes` bytes, in order. The timestamps are
/// zero so the same book gives the same bytes (the #222 rule); readers take the book's date
/// from its metadata, not from here.
pub fn header(name: &str, sizes: &[usize]) -> Result<Vec<u8>> {
    let count = u16_of(sizes.len())?;
    let mut out = Vec::with_capacity(HEADER_LEN + sizes.len() * 8 + 2);
    out.extend_from_slice(&db_name(name));
    push16(&mut out, 0); // attributes
    push16(&mut out, 0); // version
    out.extend_from_slice(&[0; 12]); // creation, modification, backup dates
    out.extend_from_slice(&[0; 12]); // modification number, app info, sort info
    out.extend_from_slice(b"BOOKMOBI");
    push32(&mut out, u32_of(2 * sizes.len())?.saturating_sub(1)); // unique id seed
    push32(&mut out, 0); // next record list
    push16(&mut out, count);
    let mut offset = HEADER_LEN + sizes.len() * 8 + 2;
    for (i, size) in sizes.iter().enumerate() {
        push32(&mut out, u32_of(offset)?);
        push32(&mut out, u32_of(2 * i)? & 0x00FF_FFFF); // attributes 0, 24-bit unique id
        offset += size;
    }
    u32_of(offset)?;
    push16(&mut out, 0);
    Ok(out)
}

/// The database name: the title in ASCII with spaces as `_`, at most 31 bytes, zero-filled.
fn db_name(title: &str) -> [u8; NAME_LEN] {
    let mut name = [0; NAME_LEN];
    let ascii = title
        .chars()
        .map(|c| match c {
            ' ' => '_',
            c if c.is_ascii_alphanumeric() || "-_.()".contains(c) => c,
            _ => '_',
        })
        .take(NAME_LEN - 1);
    for (slot, c) in name.iter_mut().zip(ascii) {
        *slot = c as u8;
    }
    if name[0] == 0 {
        name[..4].copy_from_slice(b"Book");
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn be32(b: &[u8], at: usize) -> u32 {
        u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
    }

    #[test]
    fn directory_points_at_each_record() {
        let h = header("Yotsuba&! vol 1", &[100, 20, 7]).unwrap();
        assert_eq!(h.len(), 78 + 3 * 8 + 2);
        assert_eq!(&h[..16], b"Yotsuba___vol_1\0");
        assert_eq!(&h[60..68], b"BOOKMOBI");
        assert_eq!(u16::from_be_bytes([h[76], h[77]]), 3);
        assert_eq!(be32(&h, 78), 104);
        assert_eq!(be32(&h, 86), 204);
        assert_eq!(be32(&h, 90), 2);
        assert_eq!(be32(&h, 94), 224);
    }

    #[test]
    fn long_and_empty_names() {
        assert_eq!(db_name(&"x".repeat(50))[31], 0);
        assert_eq!(&db_name("ヨツバ")[..3], b"___");
        assert_eq!(&db_name("")[..4], b"Book");
    }
}
