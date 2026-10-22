//! Big-endian integers, padding and the MOBI variable-width integer shared by the record builders.

use crate::domain::{Error, Result};

/// The "no record" value of MOBI header and index fields.
pub const NULL: u32 = 0xFFFF_FFFF;

pub fn push16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub fn push32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

/// Writes `value` at `at` in a pre-sized header.
pub fn put32(buf: &mut [u8], at: usize, value: u32) {
    if let Some(slot) = buf.get_mut(at..at + 4) {
        slot.copy_from_slice(&value.to_be_bytes());
    }
}

/// Zero bytes up to the next multiple of four.
pub fn pad4(out: &mut Vec<u8>) {
    out.resize(out.len().div_ceil(4) * 4, 0);
}

/// A count or offset as the format's 32-bit field, or an error when the book outgrows it.
pub fn u32_of(value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| Error::Other(format!("azw3: {value} does not fit in 32 bits")))
}

/// A 16-bit field, or an error when the book outgrows it.
pub fn u16_of(value: usize) -> Result<u16> {
    u16::try_from(value).map_err(|_| Error::Other(format!("azw3: {value} does not fit in 16 bits")))
}

/// The forward variable-width integer of index entries and CNCX strings: 7 bits per byte, most
/// significant first, with the high bit set on the last byte.
pub fn varint(value: u32) -> Vec<u8> {
    let mut groups = vec![(value & 0x7F) as u8 | 0x80];
    let mut rest = value >> 7;
    while rest > 0 {
        groups.push((rest & 0x7F) as u8);
        rest >>= 7;
    }
    groups.reverse();
    groups
}

/// The four-digit base-32 number (`0-9A-V`) of `kindle:embed:` and `aid` attributes.
pub fn base32(value: usize, width: usize) -> String {
    const DIGITS: &[u8; 32] = b"0123456789ABCDEFGHIJKLMNOPQRSTUV";
    let mut digits = Vec::new();
    let mut rest = value;
    loop {
        digits.push(DIGITS[rest % 32]);
        rest /= 32;
        if rest == 0 {
            break;
        }
    }
    digits.resize(digits.len().max(width), b'0');
    digits.iter().rev().map(|&d| d as char).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_sets_the_high_bit_on_the_last_byte() {
        assert_eq!(varint(0), [0x80]);
        assert_eq!(varint(0x7F), [0xFF]);
        assert_eq!(varint(0x80), [0x01, 0x80]);
        assert_eq!(varint(300), [0x02, 0xAC]);
    }

    #[test]
    fn base32_pads_to_the_width() {
        assert_eq!(base32(1, 4), "0001");
        assert_eq!(base32(31, 4), "000V");
        assert_eq!(base32(32, 4), "0010");
        assert_eq!(base32(0, 1), "0");
    }

    #[test]
    fn padding_and_fields() {
        let mut out = vec![1, 2, 3, 4, 5];
        pad4(&mut out);
        assert_eq!(out.len(), 8);
        let mut header = vec![0; 8];
        put32(&mut header, 4, 0x0102_0304);
        assert_eq!(header, [0, 0, 0, 0, 1, 2, 3, 4]);
        assert!(u16_of(70_000).is_err());
    }
}
