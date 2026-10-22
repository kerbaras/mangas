//! CNCX records: the strings index entries point to (TOC labels, fragment selectors, guide
//! titles). Each string is a forward varint length and its UTF-8 bytes; an entry refers to a
//! string by `record number × 0x10000 + offset in that record`.

use super::bytes::{u32_of, varint};
use crate::domain::Result;

const RECORD_LIMIT: usize = 0x10000 - 1024;

/// The records holding `strings`, and each string's offset in the order given.
pub fn build(strings: &[String]) -> Result<(Vec<Vec<u8>>, Vec<u32>)> {
    let mut records: Vec<Vec<u8>> = Vec::new();
    let mut offsets = Vec::with_capacity(strings.len());
    let mut current = Vec::new();
    for s in strings {
        let mut raw = varint(u32_of(s.len())?);
        raw.extend_from_slice(s.as_bytes());
        if !current.is_empty() && current.len() + raw.len() > RECORD_LIMIT {
            records.push(std::mem::take(&mut current));
        }
        offsets.push(u32_of(records.len() * 0x10000 + current.len())?);
        current.extend(raw);
    }
    if !current.is_empty() {
        super::bytes::pad4(&mut current);
        records.push(current);
    }
    for r in &mut records {
        super::bytes::pad4(r);
    }
    Ok((records, offsets))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_point_at_the_length_prefix() {
        let (records, offsets) = build(&["ab".into(), "cde".into()]).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(offsets, [0, 3]);
        assert_eq!(&records[0][..7], [0x82, b'a', b'b', 0x83, b'c', b'd', b'e']);
        assert_eq!(records[0].len() % 4, 0);
    }

    #[test]
    fn long_tables_roll_over_to_a_new_record() {
        let strings: Vec<String> = (0..400).map(|i| format!("{i:0>200}")).collect();
        let (records, offsets) = build(&strings).unwrap();
        assert_eq!(records.len(), 2);
        assert!(offsets.iter().any(|&o| o >= 0x10000));
    }
}
