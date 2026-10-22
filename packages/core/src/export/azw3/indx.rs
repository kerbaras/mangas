//! INDX records, the tables behind KF8's skeleton, fragment, NCX and guide indexes (MobileRead
//! wiki "MOBI", section "INDX"). A table is one header record (`TAGX` tag layout plus, per data
//! record, its last entry name and entry count), the data records with the entries, then the
//! table's CNCX records. An entry is its name, one control byte saying how many values each tag
//! has, and the values as forward varints in tag order.

use super::bytes::{pad4, push16, push32, put32, u16_of, u32_of, varint, NULL};
use crate::domain::{Error, Result};

const HEADER_LEN: usize = 192;
/// Room kept free in a data record, as kindlegen does, so entries never cross 64 KiB.
const RECORD_LIMIT: usize = 0x10000 - HEADER_LEN - 1048;

/// A tag of the table: its number, values per entry and its bits in the control byte.
#[derive(Debug, Clone, Copy)]
pub struct Tag {
    pub tag: u8,
    pub per_entry: u8,
    pub mask: u8,
}

/// An entry: its name and, for each tag of the table in order, its values (empty = absent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: Vec<u8>,
    pub values: Vec<Vec<u32>>,
}

/// The header record followed by the data records of a table with `cncx` CNCX records.
pub fn table(tags: &[Tag], entries: &[Entry], cncx: usize) -> Result<Vec<Vec<u8>>> {
    let mut blocks: Vec<(Vec<u8>, Vec<u16>, Vec<u8>)> = Vec::new();
    for entry in entries {
        let raw = encode(tags, entry)?;
        let full = blocks.last().is_none_or(|(body, offsets, _)| {
            body.len() + 2 * offsets.len() + raw.len() + 2 > RECORD_LIMIT
        });
        if full {
            blocks.push((Vec::new(), Vec::new(), Vec::new()));
        }
        if let Some((body, offsets, last)) = blocks.last_mut() {
            offsets.push(u16_of(HEADER_LEN + body.len())?);
            body.extend(raw);
            last.clone_from(&entry.name);
        }
    }
    let mut records = vec![header_record(tags, &blocks, entries.len(), cncx)?];
    for (body, offsets, _) in &blocks {
        records.push(data_record(body, offsets)?);
    }
    Ok(records)
}

fn encode(tags: &[Tag], entry: &Entry) -> Result<Vec<u8>> {
    let mut control = 0u8;
    let mut values = Vec::new();
    for (tag, vals) in tags.iter().zip(&entry.values) {
        let count = vals.len() / usize::from(tag.per_entry.max(1));
        let bits = u32::try_from(count).unwrap_or(u32::MAX) << tag.mask.trailing_zeros();
        if bits & !u32::from(tag.mask) != 0 {
            return Err(Error::Other(format!(
                "azw3: tag {} has {count} entries",
                tag.tag
            )));
        }
        control |= bits as u8;
        values.extend(vals.iter().flat_map(|&v| varint(v)));
    }
    let mut raw = vec![u8::try_from(entry.name.len()).map_err(|_| too_long())?];
    raw.extend_from_slice(&entry.name);
    raw.push(control);
    raw.extend(values);
    Ok(raw)
}

fn too_long() -> Error {
    Error::Other("azw3: index entry name longer than 255 bytes".into())
}

/// `INDX` with the fields shared by both record kinds; the rest are zero.
fn indx_head(kind: u32, idxt: usize, count: usize) -> Result<Vec<u8>> {
    let mut head = vec![0; HEADER_LEN];
    head[..4].copy_from_slice(b"INDX");
    put32(&mut head, 4, HEADER_LEN as u32);
    put32(&mut head, 12, kind);
    put32(&mut head, 20, u32_of(idxt)?);
    put32(&mut head, 24, u32_of(count)?);
    Ok(head)
}

fn data_record(body: &[u8], offsets: &[u16]) -> Result<Vec<u8>> {
    let mut body = body.to_vec();
    pad4(&mut body);
    let mut out = indx_head(1, HEADER_LEN + body.len(), offsets.len())?;
    out[28..36].fill(0xFF);
    out.extend(body);
    out.extend_from_slice(b"IDXT");
    offsets.iter().for_each(|&o| push16(&mut out, o));
    pad4(&mut out);
    Ok(out)
}

type Block = (Vec<u8>, Vec<u16>, Vec<u8>);

fn header_record(tags: &[Tag], blocks: &[Block], total: usize, cncx: usize) -> Result<Vec<u8>> {
    let mut tagx = b"TAGX".to_vec();
    push32(&mut tagx, u32_of(12 + 4 * (tags.len() + 1))?);
    push32(&mut tagx, 1); // control bytes per entry
    for t in tags {
        tagx.extend_from_slice(&[t.tag, t.per_entry, t.mask, 0]);
    }
    tagx.extend_from_slice(&[0, 0, 0, 1]);
    let mut geometry = Vec::new();
    let mut idxt = b"IDXT".to_vec();
    for (_, offsets, last) in blocks {
        push16(&mut idxt, u16_of(HEADER_LEN + tagx.len() + geometry.len())?);
        geometry.push(u8::try_from(last.len()).map_err(|_| too_long())?);
        geometry.extend_from_slice(last);
        push16(&mut geometry, u16_of(offsets.len())?);
    }
    pad4(&mut geometry);
    pad4(&mut idxt);
    let mut out = indx_head(0, HEADER_LEN + tagx.len() + geometry.len(), blocks.len())?;
    put32(&mut out, 16, 2);
    put32(&mut out, 28, 65001);
    put32(&mut out, 32, NULL);
    put32(&mut out, 36, u32_of(total)?);
    put32(&mut out, 52, u32_of(cncx)?);
    put32(&mut out, 180, HEADER_LEN as u32);
    out.extend(tagx);
    out.extend(geometry);
    out.extend(idxt);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAGS: [Tag; 2] = [
        Tag {
            tag: 1,
            per_entry: 1,
            mask: 0x03,
        },
        Tag {
            tag: 6,
            per_entry: 2,
            mask: 0x0C,
        },
    ];

    fn entry(name: &str) -> Entry {
        Entry {
            name: name.into(),
            values: vec![vec![1, 1], vec![0, 300, 0, 300]],
        }
    }

    #[test]
    fn entries_carry_counts_in_the_control_byte() {
        let raw = encode(&TAGS, &entry("SKEL0000000000")).unwrap();
        assert_eq!(raw[0], 14);
        assert_eq!(raw[15], 0b1010);
        assert_eq!(&raw[16..], [0x81, 0x81, 0x80, 0x02, 0xAC, 0x80, 0x02, 0xAC]);
    }

    #[test]
    fn header_then_data_record() {
        let records = table(&TAGS, &[entry("a"), entry("b")], 0).unwrap();
        assert_eq!(records.len(), 2);
        let (head, data) = (&records[0], &records[1]);
        assert_eq!(&head[192..196], b"TAGX");
        assert_eq!(u32::from_be_bytes(head[24..28].try_into().unwrap()), 1);
        assert_eq!(u32::from_be_bytes(head[36..40].try_into().unwrap()), 2);
        let idxt = u32::from_be_bytes(data[20..24].try_into().unwrap()) as usize;
        assert_eq!(&data[idxt..idxt + 4], b"IDXT");
        assert_eq!(u16::from_be_bytes([data[idxt + 4], data[idxt + 5]]), 192);
    }

    #[test]
    fn too_many_values_for_the_mask_fail() {
        let bad = Entry {
            name: b"x".to_vec(),
            values: vec![vec![1; 4], vec![]],
        };
        assert!(encode(&TAGS, &bad).is_err());
    }
}
