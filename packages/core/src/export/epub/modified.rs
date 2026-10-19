//! `dcterms:modified` from the book itself, not the clock: the same pages and metadata always
//! give the same date, so two exports of the same chapters are byte-identical, and any change
//! to what is in the book gives another date (EPUB readers treat a new date as a new revision).

use sha2::{Digest, Sha256};

use super::Book;

/// 2000-01-01T00:00:00Z: the dates fall in the twenty years after it.
const EPOCH: i64 = 946_684_800;
const SPAN: u64 = 20 * 365 * 24 * 3600;

pub(super) fn modified(book: &Book) -> String {
    let mut hash = Sha256::new();
    for text in [&book.id, &book.title, &book.language] {
        hash.update(text.as_bytes());
        hash.update([0]);
    }
    for author in &book.authors {
        hash.update(author.as_bytes());
        hash.update([0]);
    }
    for page in book.cover.iter().chain(&book.pages) {
        hash.update(page.width.to_le_bytes());
        hash.update(page.height.to_le_bytes());
        hash.update(&page.jpeg);
    }
    let digest = hash.finalize();
    let mut head = [0u8; 8];
    head.copy_from_slice(&digest[..8]);
    let seconds = EPOCH + (u64::from_le_bytes(head) % SPAN) as i64;
    chrono::DateTime::from_timestamp(seconds, 0)
        .unwrap_or_default()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::epub::PageImage;

    fn book(jpeg: &[u8]) -> Book {
        Book {
            id: "id".into(),
            title: "Book".into(),
            authors: vec!["A".into()],
            language: "en".into(),
            series: None,
            right_to_left: false,
            device_width: 10,
            device_height: 10,
            chapters: Vec::new(),
            pages: vec![PageImage {
                jpeg: jpeg.to_vec(),
                width: 1,
                height: 1,
            }],
            cover: None,
        }
    }

    #[test]
    fn the_same_book_gets_the_same_date_and_other_pages_another() {
        let date = modified(&book(b"one"));
        assert_eq!(date, modified(&book(b"one")));
        assert_ne!(date, modified(&book(b"two")));
        assert_eq!(date.len(), "2000-01-01T00:00:00Z".len());
        assert!(
            date.as_str() >= "2000-01-01" && date.as_str() < "2020-01-01",
            "{date}"
        );
    }
}
