//! The four KF8 indexes over the text: SKEL (one entry per file: fragment count and the
//! skeleton's span), FRAG (one per fragment: insert position, parent selector, file, sequence
//! number, span), NCX (the chapter TOC, pointing at fragments) and the guide (start reading).
//! Tag layouts as kindlegen writes them (MobileRead wiki "KF8", section "Indexes").

use super::bytes::u32_of;
use super::cncx;
use super::indx::{table, Entry, Tag};
use super::text::File;
use crate::domain::Result;

const fn tag(tag: u8, per_entry: u8, mask: u8) -> Tag {
    Tag {
        tag,
        per_entry,
        mask,
    }
}

/// A TOC entry: its label and the file it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocEntry {
    pub label: String,
    pub file: usize,
}

pub fn skeleton(files: &[File]) -> Result<Vec<Vec<u8>>> {
    let tags = [tag(1, 1, 0x03), tag(6, 2, 0x0C)];
    let mut entries = Vec::new();
    for (i, f) in files.iter().enumerate() {
        let (start, len) = (u32_of(f.start)?, u32_of(f.skeleton_len)?);
        entries.push(Entry {
            name: format!("SKEL{i:010}").into_bytes(),
            // kindlegen repeats both values; readers take the first.
            values: vec![vec![1, 1], vec![start, len, start, len]],
        });
    }
    table(&tags, &entries, 0)
}

/// The FRAG table and its CNCX records (the selectors).
pub fn fragment(files: &[File]) -> Result<Vec<Vec<u8>>> {
    let tags = [
        tag(2, 1, 0x01),
        tag(3, 1, 0x02),
        tag(4, 1, 0x04),
        tag(6, 2, 0x08),
    ];
    let selectors: Vec<String> = files.iter().map(|f| f.selector.clone()).collect();
    let (cncx, offsets) = cncx::build(&selectors)?;
    let mut entries = Vec::new();
    for (i, (f, offset)) in files.iter().zip(offsets).enumerate() {
        let n = u32_of(i)?;
        entries.push(Entry {
            name: format!("{:010}", f.insert).into_bytes(),
            values: vec![
                vec![offset],
                vec![n],
                vec![n],
                vec![0, u32_of(f.fragment_len)?],
            ],
        });
    }
    with_cncx(table(&tags, &entries, cncx.len())?, cncx)
}

/// The NCX: one depth-0 entry per chapter, spanning the text up to the next chapter.
pub fn ncx(files: &[File], text_len: usize, toc: &[TocEntry]) -> Result<Vec<Vec<u8>>> {
    let tags = [
        tag(1, 1, 0x01),
        tag(2, 1, 0x02),
        tag(3, 1, 0x04),
        tag(4, 1, 0x08),
        tag(21, 1, 0x10),
        tag(22, 1, 0x20),
        tag(23, 1, 0x40),
        tag(6, 2, 0x80),
    ];
    let labels: Vec<String> = toc.iter().map(|t| t.label.clone()).collect();
    let (cncx, offsets) = cncx::build(&labels)?;
    let width = format!("{:X}", toc.len().saturating_sub(1)).len().max(2);
    let start = |file: usize| files.get(file).map_or(text_len, |f| f.start);
    let mut entries = Vec::new();
    for (i, (t, label)) in toc.iter().zip(offsets).enumerate() {
        let end = toc.get(i + 1).map_or(text_len, |next| start(next.file));
        let pos = start(t.file);
        entries.push(Entry {
            name: format!("{i:0width$X}").into_bytes(),
            values: vec![
                vec![u32_of(pos)?],
                vec![u32_of(end.saturating_sub(pos))?],
                vec![label],
                vec![0],
                vec![],
                vec![],
                vec![],
                vec![u32_of(t.file)?, 0],
            ],
        });
    }
    with_cncx(table(&tags, &entries, cncx.len())?, cncx)
}

/// The guide: `text`, where reading starts (the first file).
pub fn guide() -> Result<Vec<Vec<u8>>> {
    let tags = [tag(1, 1, 0x01), tag(6, 2, 0x02)];
    let (cncx, offsets) = cncx::build(&["Beginning".to_string()])?;
    let entries = [Entry {
        name: b"text".to_vec(),
        values: vec![offsets, vec![0, 0]],
    }];
    with_cncx(table(&tags, &entries, cncx.len())?, cncx)
}

fn with_cncx(mut records: Vec<Vec<u8>>, cncx: Vec<Vec<u8>>) -> Result<Vec<Vec<u8>>> {
    records.extend(cncx);
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(start: usize) -> File {
        File {
            start,
            skeleton_len: 10,
            insert: start + 6,
            fragment_len: 5,
            selector: "P-//*[@aid='0']".into(),
        }
    }

    #[test]
    fn each_table_is_header_data_then_cncx() {
        let files = [file(0), file(15)];
        assert_eq!(skeleton(&files).unwrap().len(), 2);
        let frag = fragment(&files).unwrap();
        assert_eq!(frag.len(), 3);
        assert!(frag[1].windows(10).any(|w| w == b"0000000021"));
        let toc = [
            TocEntry {
                label: "Ch 1".into(),
                file: 0,
            },
            TocEntry {
                label: "Ch 2".into(),
                file: 1,
            },
        ];
        assert_eq!(ncx(&files, 30, &toc).unwrap().len(), 3);
        assert_eq!(guide().unwrap().len(), 3);
    }
}
