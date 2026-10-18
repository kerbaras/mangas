use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::{Availability, MangaId};

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChapterId(pub String);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chapter {
    pub id: ChapterId,
    pub manga_id: MangaId,
    pub title: Option<String>,
    pub language: String,
    pub volume: Option<String>,
    /// Chapter number as published ("12", "12.5"); kept as text to preserve the source's labelling.
    pub number: String,
    /// Local directory with the downloaded pages, once downloaded.
    pub local_path: Option<String>,
    /// Scanlation group(s) credited by the source ("A, B" when several).
    #[serde(default)]
    pub group: Option<String>,
    /// Whether the source lets this chapter be downloaded.
    #[serde(default)]
    pub availability: Availability,
    /// When the chapter was last part of a successful export (RFC 3339, UTC); the library sets it.
    #[serde(default)]
    pub exported_at: Option<String>,
}

impl Chapter {
    pub fn is_downloaded(&self) -> bool {
        self.local_path.is_some()
    }

    pub fn is_exported(&self) -> bool {
        self.exported_at.is_some()
    }

    /// The source lets this chapter be downloaded.
    pub fn is_available(&self) -> bool {
        self.availability.is_available()
    }

    /// Numeric value of [`Chapter::number`], if it parses ("12.5" -> 12.5).
    pub fn number_value(&self) -> Option<f64> {
        parse_number(&self.number)
    }
}

/// Parses a published chapter number. Non-numeric labels ("Oneshot") yield `None`.
pub fn parse_number(number: &str) -> Option<f64> {
    number.trim().parse::<f64>().ok().filter(|n| n.is_finite())
}

/// Orders chapters by number ascending; unparsable numbers go last, ties keep their order.
pub fn compare_numbers(a: &str, b: &str) -> Ordering {
    match (parse_number(a), parse_number(b)) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.cmp(b),
    }
}

/// Stable sort by chapter number ascending.
pub fn sort_by_number(mut chapters: Vec<Chapter>) -> Vec<Chapter> {
    chapters.sort_by(|a, b| compare_numbers(&a.number, &b.number));
    chapters
}

impl std::fmt::Display for ChapterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
