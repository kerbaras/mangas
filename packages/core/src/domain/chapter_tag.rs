//! How file names and tables of contents name a chapter, so every export agrees.
//!
//! Season-numbered titles (see [`super::season_label`]) carry the number readers know
//! (`S2 Ep. 0`), so it replaces the absolute chapter number (`80`) wherever a chapter is named.

use super::Chapter;

/// The leading `S2 Ep. 5`, `S2 Prologue` or `S2 Part 1` of a season-labelled title; `None` for
/// any other title, including a bare `S2` that names no episode.
pub fn season_tag(title: &str) -> Option<&str> {
    let title = title.trim_start();
    let rest = title.strip_prefix('S')?;
    let season = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let rest = rest[season..].strip_prefix(' ').filter(|_| season > 0)?;
    let tail = match rest.strip_prefix("Ep. ") {
        Some(ep) => number_len(ep).map(|n| &ep[n..]),
        None => word_len(rest).map(|n| numbered(&rest[n..])),
    }?;
    (tail.is_empty() || tail.starts_with(' ')).then(|| &title[..title.len() - tail.len()])
}

/// The short name of `c` in a file name: its season tag, else `ch 12`.
pub fn chapter_tag(c: &Chapter) -> String {
    c.title
        .as_deref()
        .and_then(season_tag)
        .map_or_else(|| format!("ch {}", c.number.trim()), str::to_string)
}

/// The table-of-contents entry: a season-labelled title as is, else "Chapter 12: The title",
/// "Chapter 12", or the bare title for unnumbered chapters.
pub fn toc_label(number: &str, title: Option<&str>) -> String {
    let number = number.trim();
    let title = title.map(str::trim).filter(|t| !t.is_empty());
    match (number.is_empty(), title) {
        (_, Some(t)) if season_tag(t).is_some() => t.to_string(),
        (false, Some(t)) => format!("Chapter {number}: {t}"),
        (false, None) => format!("Chapter {number}"),
        (true, Some(t)) => t.to_string(),
        (true, None) => "Chapter".to_string(),
    }
}

/// Length of a leading `12` or `12.5`.
fn number_len(s: &str) -> Option<usize> {
    let digits = |s: &str| s.len() - s.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let whole = digits(s);
    if whole == 0 {
        return None;
    }
    let fraction = s[whole..].strip_prefix('.').map(digits).filter(|&n| n > 0);
    Some(whole + fraction.map_or(0, |n| n + 1))
}

/// `rest` after an optional ` 2` that numbers the word before it (`Part 2`).
fn numbered(rest: &str) -> &str {
    let after = rest.strip_prefix(' ').unwrap_or(" ");
    match number_len(after) {
        Some(n) if after[n..].is_empty() || after[n..].starts_with(' ') => &after[n..],
        _ => rest,
    }
}

/// Length of a leading word of letters (`Prologue`).
fn word_len(s: &str) -> Option<usize> {
    let n = s.len() - s.trim_start_matches(|c: char| c.is_alphabetic()).len();
    (n > 0).then_some(n)
}
