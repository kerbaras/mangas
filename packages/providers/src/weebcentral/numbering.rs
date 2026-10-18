//! Chapter numbers from the site's labels. The core orders chapters by number, so every chapter
//! needs one that sorts where the site lists it (the site's order is the reading order).

use std::collections::HashSet;

use mangas_core::domain::{parse_number, sort_by_number, Chapter, ChapterId, MangaId};

use super::chapters::{parse_label, Label};

/// Chapters from `(id, label)` pairs in the site's order (newest first), sorted by number.
/// "Chapter N" keeps N. Once seasons appear the label numbers repeat, so every chapter is
/// numbered by its position in reading order (oldest = 1) and keeps its label as the title.
/// Unnumbered labels ("Prologue 4") keep the label as the title and get a number between
/// their neighbours (see [`fill_gaps`]).
pub fn number_chapters(listed: Vec<(ChapterId, String)>, manga: &MangaId) -> Vec<Chapter> {
    let mut seen = HashSet::new();
    let reading: Vec<_> = listed
        .into_iter()
        .rev()
        .filter(|(id, _)| seen.insert(id.clone()))
        .collect();
    let labels: Vec<Label> = reading.iter().map(|(_, l)| parse_label(l)).collect();
    let seasonal = labels.iter().any(|l| matches!(l, Label::Season { .. }));
    let (numbers, titles): (Vec<_>, Vec<_>) = reading
        .iter()
        .zip(labels)
        .enumerate()
        .map(|(i, ((_, raw), label))| match label {
            _ if seasonal => (Some((i + 1).to_string()), Some(raw.clone())),
            Label::Numbered { number, title } => (Some(number), title),
            Label::Season { .. } | Label::Other => (None, Some(raw.clone())),
        })
        .unzip();
    let chapters = reading
        .into_iter()
        .zip(fill_gaps(numbers))
        .zip(titles)
        .map(|(((id, _), number), title)| Chapter {
            id,
            manga_id: manga.clone(),
            title: title.filter(|t| !t.trim().is_empty()),
            language: "en".to_string(),
            volume: None,
            number,
            local_path: None,
            group: None,
        })
        .collect();
    sort_by_number(chapters)
}

/// Numbers in reading order with the gaps filled: a run of `None` is spread evenly between
/// the numbers around it (a run before the first number ends there, one after the last starts
/// there; each spans one unit). `[None, None, "1"]` gives `["0.33", "0.67", "1"]`.
pub fn fill_gaps(numbers: Vec<Option<String>>) -> Vec<String> {
    let values: Vec<Option<f64>> = numbers
        .iter()
        .map(|n| n.as_deref().and_then(parse_number))
        .collect();
    numbers
        .into_iter()
        .enumerate()
        .map(|(i, n)| match n.filter(|_| values[i].is_some()) {
            Some(n) => n,
            None => gap_number(&values, i),
        })
        .collect()
}

fn gap_number(values: &[Option<f64>], i: usize) -> String {
    let start = values[..i]
        .iter()
        .rposition(Option::is_some)
        .map_or(0, |p| p + 1);
    let end = values[i..]
        .iter()
        .position(Option::is_some)
        .map_or(values.len(), |p| i + p);
    let before = start.checked_sub(1).and_then(|p| values[p]);
    let after = values.get(end).copied().flatten();
    let (low, high) = match (before, after) {
        (Some(b), Some(a)) if a > b => (b, a),
        (Some(b), _) => (b, b + 1.0),
        (None, Some(a)) => (a - 1.0, a),
        (None, None) => (0.0, 1.0),
    };
    let run = end - start;
    let value = low + (high - low) * (i - start + 1) as f64 / (run + 1) as f64;
    let decimals = run.to_string().len() + 1;
    let text = format!("{value:.decimals$}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}
