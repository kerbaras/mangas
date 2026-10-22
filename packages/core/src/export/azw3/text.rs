//! The KF8 text: one XHTML file per page (the cover first when the book has one), each split
//! into a skeleton (the document around `<body>`) and one fragment (the page's image) the way
//! the SKEL and FRAG indexes describe it. The text is plain ASCII, non-ASCII characters
//! written as character references, so a 4096-byte text record never splits a character.

use super::bytes::base32;

/// Text records hold this many bytes of the text (MOBI's PalmDOC record size).
pub const RECORD_SIZE: usize = 4096;

/// A page of the text: its image's 1-based `kindle:embed` number, size and label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub image: usize,
    pub width: u32,
    pub height: u32,
    pub label: String,
}

/// Where a page's file sits in the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub start: usize,
    pub skeleton_len: usize,
    /// Absolute position in the text where the fragment goes into the skeleton.
    pub insert: usize,
    pub fragment_len: usize,
    /// The fragment's parent element, `P-//*[@aid='…']`.
    pub selector: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub bytes: Vec<u8>,
    pub files: Vec<File>,
}

/// The text of `pages`: each file is its skeleton followed by its fragment.
pub fn build(title: &str, pages: &[Page]) -> Text {
    let mut bytes = Vec::new();
    let mut files = Vec::with_capacity(pages.len());
    for (i, page) in pages.iter().enumerate() {
        let body_aid = base32(2 * i, 1);
        let (head, tail) = skeleton(title, page, &body_aid);
        let fragment = fragment(page, &base32(2 * i + 1, 1));
        let start = bytes.len();
        bytes.extend_from_slice(head.as_bytes());
        bytes.extend_from_slice(tail.as_bytes());
        bytes.extend_from_slice(fragment.as_bytes());
        files.push(File {
            start,
            skeleton_len: head.len() + tail.len(),
            insert: start + head.len(),
            fragment_len: fragment.len(),
            selector: format!("P-//*[@aid='{body_aid}']"),
        });
    }
    Text { bytes, files }
}

/// The skeleton, cut where the fragment goes in.
fn skeleton(title: &str, p: &Page, aid: &str) -> (String, String) {
    let (w, h) = (p.width, p.height);
    let head = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n\
<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\
<head><title>{t} - {label}</title><meta name=\"viewport\" content=\"width={w}, height={h}\"/>\
<style type=\"text/css\">html, body {{ margin: 0; padding: 0; width: {w}px; height: {h}px; }} \
img {{ display: block; width: {w}px; height: {h}px; }}</style></head><body aid=\"{aid}\">",
        t = escape(title),
        label = escape(&p.label),
    );
    (head, "</body></html>".into())
}

fn fragment(p: &Page, aid: &str) -> String {
    format!(
        "<div aid=\"{aid}\"><img src=\"kindle:embed:{n}?mime=image/jpeg\" alt=\"{label}\"/></div>",
        n = base32(p.image, 4),
        label = escape(&p.label),
    )
}

/// XML-escaped, every non-ASCII character as a numeric character reference.
pub fn escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            c if c.is_ascii() && !c.is_ascii_control() => c.to_string(),
            c => format!("&#x{:X};", c as u32),
        })
        .collect()
}

/// The text cut into 4096-byte records, each ending with the multibyte trailing entry (one
/// byte, zero overlap bytes: the text is ASCII), as MOBI header extra flag bit 0 announces.
pub fn records(text: &[u8]) -> Vec<Vec<u8>> {
    text.chunks(RECORD_SIZE)
        .map(|chunk| {
            let mut record = chunk.to_vec();
            record.push(0);
            record
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(image: usize) -> Page {
        Page {
            image,
            width: 600,
            height: 872,
            label: format!("Page {image}"),
        }
    }

    #[test]
    fn fragments_go_into_the_skeleton_body() {
        let text = build("Yotsuba&! ヨ", &[page(1), page(2)]);
        assert!(text.bytes.is_ascii());
        let second = &text.files[1];
        let file =
            &text.bytes[second.start..second.start + second.skeleton_len + second.fragment_len];
        let file = std::str::from_utf8(file).unwrap();
        let at = second.insert - second.start;
        assert!(file[..at].ends_with("<body aid=\"2\">"));
        assert!(file[second.skeleton_len..]
            .starts_with("<div aid=\"3\"><img src=\"kindle:embed:0002?mime=image/jpeg\""));
        assert!(file.contains("Yotsuba&amp;! &#x30E8; - Page 2"));
        assert_eq!(second.selector, "P-//*[@aid='2']");
        assert_eq!(
            second.start,
            text.files[0].start + text.files[0].skeleton_len + text.files[0].fragment_len
        );
    }

    #[test]
    fn records_have_the_trailing_byte() {
        let r = records(&[b'a'; 5000]);
        assert_eq!(r.iter().map(Vec::len).collect::<Vec<_>>(), [4097, 905]);
        assert_eq!(r[0][4096], 0);
    }
}
