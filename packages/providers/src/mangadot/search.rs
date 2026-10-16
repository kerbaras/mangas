//! Title matching for MangaDot: the API lists titles by page and the text filter is applied
//! locally, so a search never depends on an undocumented query parameter.

use mangas_core::domain::Manga;

/// Case-insensitive match: every word of `query` appears in `title`.
pub fn matches(title: &str, query: &str) -> bool {
    let title = title.to_lowercase();
    query
        .split_whitespace()
        .all(|word| title.contains(&word.to_lowercase()))
}

/// Keeps the titles matching `query`, at most `limit`, in the order given.
pub fn filter(items: Vec<Manga>, query: &str, limit: usize) -> Vec<Manga> {
    items
        .into_iter()
        .filter(|m| matches(&m.title, query))
        .take(limit)
        .collect()
}

/// True when the API honoured the text query: at least one result matches it.
pub fn honoured(items: &[Manga], query: &str) -> bool {
    items.iter().any(|m| matches(&m.title, query))
}
