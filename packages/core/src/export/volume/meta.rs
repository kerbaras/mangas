//! EPUB metadata of a volume, taken from the manga's details.

use std::path::PathBuf;

use super::Volume;
use crate::domain::{toc_label, Chapter, Manga};
use crate::export::epub::Series;
use crate::export::kindle::BookMeta;

/// The table-of-contents entry of `c` (see [`toc_label`]).
pub fn chapter_label(c: &Chapter) -> String {
    toc_label(&c.number, c.title.as_deref())
}

/// The language every chapter shares, if they agree.
fn language(chapters: &[Chapter]) -> Option<String> {
    let first = chapters.first()?.language.trim();
    (!first.is_empty() && chapters.iter().all(|c| c.language.trim() == first))
        .then(|| first.to_string())
}

/// Authors, then artists not already credited; a book needs at least one creator, so a manga
/// without credits leaves the list empty for the exporter's default.
fn creators(manga: &Manga) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in manga.authors.iter().chain(&manga.artists) {
        let name = name.trim();
        if !name.is_empty() && !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
    }
    out
}

/// Provider, series and chapter set: the same chapters always make the same book.
fn identity(manga: &Manga, chapters: &[Chapter]) -> String {
    let ids: Vec<&str> = chapters.iter().map(|c| c.id.0.as_str()).collect();
    format!("{}/{}/{}", manga.source, manga.id.0, ids.join(","))
}

pub fn book_meta(manga: &Manga, volume: &Volume, cover: Option<PathBuf>) -> BookMeta {
    BookMeta {
        authors: creators(manga),
        language: language(&volume.chapters),
        series: Some(Series {
            name: manga.title.trim().to_string(),
            position: volume.number.clone(),
        }),
        cover,
        chapter_titles: volume.chapters.iter().map(chapter_label).collect(),
        identity: Some(identity(manga, &volume.chapters)),
    }
}
