//! The small fixed records after the indexes: FDST (the text's flow table), FLIS, FCIS and the
//! end-of-file marker. Layouts from the MobileRead wiki "KF8" and "MOBI" pages; FLIS and FCIS
//! carry the constants every kindlegen file has.

use super::bytes::{push16, push32};

pub const EOF: [u8; 4] = [0xE9, 0x8E, 0x0D, 0x0A];

/// One flow: the whole XHTML text (pages style themselves inline, so there is no CSS flow).
pub fn fdst(text_len: u32) -> Vec<u8> {
    let mut out = b"FDST".to_vec();
    push32(&mut out, 12); // offset of the flow table
    push32(&mut out, 1);
    push32(&mut out, 0);
    push32(&mut out, text_len);
    out
}

pub fn flis() -> Vec<u8> {
    let mut out = b"FLIS".to_vec();
    push32(&mut out, 8);
    push16(&mut out, 65);
    push16(&mut out, 0);
    push32(&mut out, 0);
    push32(&mut out, 0xFFFF_FFFF);
    push16(&mut out, 1);
    push16(&mut out, 3);
    push32(&mut out, 3);
    push32(&mut out, 1);
    push32(&mut out, 0xFFFF_FFFF);
    out
}

/// FCIS for a single-flow text of `text_len` bytes.
pub fn fcis(text_len: u32) -> Vec<u8> {
    let mut out = b"FCIS".to_vec();
    for value in [20, 16, 1, 0, text_len, 0, 0x20, 8] {
        push32(&mut out, value);
    }
    push16(&mut out, 1);
    push16(&mut out, 1);
    push32(&mut out, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_sizes() {
        assert_eq!(fdst(10).len(), 20);
        assert_eq!(&fdst(10)[16..], 10u32.to_be_bytes());
        assert_eq!(flis().len(), 36);
        assert_eq!(fcis(7).len(), 44);
        assert_eq!(&fcis(7)[20..24], 7u32.to_be_bytes());
    }
}
