//! What an import did, for the CLI output and the desktop status line.

use std::path::PathBuf;

use serde::Serialize;

use crate::domain::Manga;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportedChapter {
    /// Parsed chapter number; empty when the name had none.
    pub number: String,
    pub title: Option<String>,
    pub pages: usize,
    pub path: PathBuf,
}

/// A file or folder left out of the import, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skipped {
    pub name: String,
    pub reason: String,
}

impl Skipped {
    pub fn new(name: &str, reason: &str) -> Self {
        Skipped {
            name: name.to_string(),
            reason: reason.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportReport {
    pub manga: Manga,
    pub imported: Vec<ImportedChapter>,
    pub skipped: Vec<Skipped>,
}

impl ImportReport {
    pub fn pages(&self) -> usize {
        self.imported.iter().map(|c| c.pages).sum()
    }

    /// Imported chapters whose name carried no number.
    pub fn unnumbered(&self) -> usize {
        self.imported.iter().filter(|c| c.number.is_empty()).count()
    }

    /// One line: `Imported 3 chapters (42 pages) of Title; 1 skipped`.
    pub fn summary(&self) -> String {
        let n = self.imported.len();
        let mut line = if n == 0 {
            format!("Nothing new in {}", self.manga.title)
        } else {
            let pages = self.pages();
            format!(
                "Imported {n} {} ({pages} {}) of {}",
                plural(n, "chapter"),
                plural(pages, "page"),
                self.manga.title
            )
        };
        if self.unnumbered() > 0 {
            line.push_str(&format!("; {} without a number", self.unnumbered()));
        }
        if !self.skipped.is_empty() {
            line.push_str(&format!("; {} skipped", self.skipped.len()));
        }
        line
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        word.to_string()
    } else {
        format!("{word}s")
    }
}
