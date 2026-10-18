//! EPUB metadata of a volume, taken from the manga's details.

use std::path::PathBuf;

use super::Volume;
use crate::domain::{Chapter, Manga};
use crate::export::epub::Series;
use crate::export::kindle::BookMeta;

/// "Chapter 12: The title", "Chapter 12", or the bare title for unnumbered chapters.
pub fn chapter_label(c: &Chapter) -> String {
    let number = c.number.trim();
    let title = c.title.as_deref().map(str::trim).filter(|t| !t.is_empty());
    match (number.is_empty(), title) {
        (false, Some(t)) => format!("Chapter {number}: {t}"),
        (false, None) => format!("Chapter {number}"),
        (true, Some(t)) => t.to_string(),
        (true, None) => "Chapter".to_string(),
    }
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
