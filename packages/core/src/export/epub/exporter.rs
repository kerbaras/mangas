//! EPUB behind the [`Exporter`] contract: the same fixed-layout book the Kindle export wrote.

use std::path::{Path, PathBuf};

use super::{write, Book};
use crate::domain::Result;
use crate::export::{Exporter, Meta, PageSpool, Profile};

pub struct EpubExporter;

impl Exporter for EpubExporter {
    fn name(&self) -> &'static str {
        "EPUB"
    }

    fn extension(&self) -> &'static str {
        "epub"
    }

    fn export(
        &self,
        pages: PageSpool,
        meta: &Meta,
        profile: &Profile,
        output: &Path,
        replace: bool,
    ) -> Result<PathBuf> {
        let book = Book {
            id: meta.id.clone(),
            title: meta.title.clone(),
            authors: creators(&meta.authors),
            language: meta.language.clone(),
            series: meta.series.clone(),
            right_to_left: meta.right_to_left,
            device_width: profile.width,
            device_height: profile.height,
            chapters: meta.chapters.clone(),
            pages,
            cover: meta.cover.clone(),
        };
        write(&book, output, replace)
    }
}

/// `authors`, or "mangas" when none is known: EPUB readers show a creator line.
fn creators(authors: &[String]) -> Vec<String> {
    match authors.is_empty() {
        true => vec!["mangas".into()],
        false => authors.to_vec(),
    }
}
