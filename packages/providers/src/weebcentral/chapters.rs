//! `/series/{id}/full-chapter-list` (HTMX fragment): every chapter, newest first, as
//! `<a href="/chapters/{id}">` with its label ("Chapter 12.5", "Episode 3", "S2 - Chapter 5",
//! "Prologue 4") in the first `span.grow > span`.

use std::collections::HashSet;

use mangas_core::domain::{parse_number, sort_by_number, Chapter, ChapterId, MangaId, Result};

use super::series::segment_after;
use crate::html;

/// What a chapter label says about its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    /// "Chapter 12.5", "Episode 3 - Title": the series-wide number, plus any title after it.
    Numbered {
        number: String,
        title: Option<String>,
    },
    /// "S2 - Chapter 5": numbering restarts every season.
    Season { season: u32, number: String },
    /// Anything else ("Prologue 4", "Oneshot"): no series-wide number.
    Other,
}

const NUMBER_WORDS: &[&str] = &["chapter", "episode", "ch.", "ep.", "ch", "ep"];

pub fn parse_label(raw: &str) -> Label {
    let raw = raw.trim();
    if let Some((season, rest)) = season_prefix(raw) {
        return match parse_label(rest) {
            Label::Numbered { number, .. } => Label::Season { season, number },
            _ => Label::Other,
        };
    }
    let mut words = raw.splitn(3, char::is_whitespace);
    let (Some(word), Some(number)) = (words.next(), words.next()) else {
        return Label::Other;
    };
    if !NUMBER_WORDS.contains(&word.to_ascii_lowercase().as_str()) || parse_number(number).is_none()
    {
        return Label::Other;
    }
    let title = words
        .next()
        .map(|t| {
            t.trim_start_matches([' ', '-', ':', '–'])
                .trim()
                .to_string()
        })
        .filter(|t| !t.is_empty());
    Label::Numbered {
        number: number.to_string(),
        title,
    }
}

/// `S<digits> - <rest>`.
fn season_prefix(raw: &str) -> Option<(u32, &str)> {
    let (head, rest) = raw.split_once(" - ")?;
    let season = head.strip_prefix(['S', 's'])?.parse().ok()?;
    Some((season, rest))
}

pub fn parse_chapters(body: &str, manga: &MangaId) -> Result<Vec<Chapter>> {
    let doc = html::parse(body);
    let mut listed = Vec::new();
    for link in html::select(html::root(&doc), r#"a[href*="/chapters/"]"#)? {
        let Some(id) = html::attr(link, "href").and_then(|h| segment_after(&h, "chapters")) else {
            continue;
        };
        let label = html::first(link, "span.grow > span")?
            .map(html::text)
            .unwrap_or_default();
        listed.push((ChapterId(id), label));
    }
    Ok(number_chapters(listed, manga))
}

/// Chapters from `(id, label)` pairs in the site's order (newest first), sorted by number.
/// Labels with a series-wide number keep it. Once seasons appear the label numbers repeat, so
/// every chapter is numbered by its position in reading order (oldest = 1, like the absolute
/// episode numbers of the official releases) and keeps its label as the title. Other labels
/// get no number and keep the label as the title.
pub fn number_chapters(listed: Vec<(ChapterId, String)>, manga: &MangaId) -> Vec<Chapter> {
    let mut seen = HashSet::new();
    let listed: Vec<_> = listed
        .into_iter()
        .filter(|(id, _)| seen.insert(id.clone()))
        .collect();
    let labels: Vec<Label> = listed.iter().map(|(_, l)| parse_label(l)).collect();
    let seasonal = labels.iter().any(|l| matches!(l, Label::Season { .. }));
    let total = listed.len();
    let chapters = listed
        .into_iter()
        .zip(labels)
        .enumerate()
        .map(|(i, ((id, raw), label))| {
            let (number, title) = match label {
                _ if seasonal => ((total - i).to_string(), Some(raw)),
                Label::Numbered { number, title } => (number, title),
                Label::Season { .. } | Label::Other => (String::new(), Some(raw)),
            };
            Chapter {
                id,
                manga_id: manga.clone(),
                title: title.filter(|t| !t.trim().is_empty()),
                language: "en".to_string(),
                volume: None,
                number,
                local_path: None,
                group: None,
            }
        })
        .collect();
    sort_by_number(chapters)
}
