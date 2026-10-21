//! `dcterms:modified` from the book itself, not the clock: the same pages and metadata always
//! give the same date, so two exports of the same chapters are byte-identical, and any change
//! to what is in the book gives another date (EPUB readers treat a new date as a new revision).

use sha2::{Digest, Sha256};

use super::{Book, PageImage};
use crate::domain::Result;

/// 2000-01-01T00:00:00Z: the dates fall in the twenty years after it.
const EPOCH: i64 = 946_684_800;
const SPAN: u64 = 20 * 365 * 24 * 3600;

/// Reads the pages back from the spool one at a time to hash them.
pub(super) fn modified(book: &Book) -> Result<String> {
    let mut hash = Sha256::new();
    let mut text = |t: &str| {
        hash.update(t.as_bytes());
        hash.update([0]);
    };
    text(&book.id);
    text(&book.title);
    text(&book.language);
    book.authors.iter().for_each(|a| text(a));
    text("series");
    if let Some(series) = &book.series {
        text(&series.name);
        text(series.position.as_deref().unwrap_or(""));
    }
    text("chapters");
    for chapter in &book.chapters {
        text(&chapter.title);
        text(&chapter.first_page.to_string());
    }
    hash.update([u8::from(book.right_to_left)]);
    hash.update(book.device_width.to_le_bytes());
    hash.update(book.device_height.to_le_bytes());
    hash.update([u8::from(book.cover.is_some())]);
    let mut page = |page: &PageImage| {
        hash.update(page.width.to_le_bytes());
        hash.update(page.height.to_le_bytes());
        hash.update((page.jpeg.len() as u64).to_le_bytes());
        hash.update(&page.jpeg);
    };
    book.cover.iter().for_each(&mut page);
    for i in 0..book.pages.len() {
        page(&book.pages.read(i)?);
    }
    let digest = hash.finalize();
    let mut head = [0u8; 8];
    head.copy_from_slice(&digest[..8]);
    let seconds = EPOCH + (u64::from_le_bytes(head) % SPAN) as i64;
    Ok(chrono::DateTime::from_timestamp(seconds, 0)
        .unwrap_or_default()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::epub::{ChapterEntry, Series};
    use crate::export::PageSpool;

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
            pages: PageSpool::from_pages([PageImage {
                jpeg: jpeg.to_vec(),
                width: 1,
                height: 1,
            }])
            .unwrap(),
            cover: None,
        }
    }

    fn date_of(book: &Book) -> String {
        modified(book).unwrap()
    }

    #[test]
    fn the_same_book_gets_the_same_date_and_other_pages_another() {
        let date = date_of(&book(b"one"));
        assert_eq!(date, date_of(&book(b"one")));
        assert_ne!(date, date_of(&book(b"two")));
        let mut renamed = book(b"one");
        renamed.chapters = vec![ChapterEntry {
            title: "Chapter 1".into(),
            first_page: 0,
        }];
        assert_ne!(date, date_of(&renamed), "a chapter title is metadata too");
        let mut series = book(b"one");
        series.series = Some(Series {
            name: "S".into(),
            position: Some("2".into()),
        });
        assert_ne!(date, date_of(&series), "so is the series");
        assert_eq!(date.len(), "2000-01-01T00:00:00Z".len());
        assert!(
            date.as_str() >= "2000-01-01" && date.as_str() < "2020-01-01",
            "{date}"
        );
    }
}
