//! Matching the user's chapter languages to the ones a provider publishes. A regional code
//! falls back to its base language when the provider has no edition for the region: a saved
//! `es-la` reads Webtoons' plain `es`.

use crate::registry;

/// The code `offered` uses for `wanted`: the exact code when offered (ASCII case ignored), else
/// its base language (`es-la` -> `es`, `pt-br` -> `pt`) when that is offered, else `None`.
pub fn offered_language<'a>(wanted: &str, offered: &[&'a str]) -> Option<&'a str> {
    let find = |code: &str| {
        offered
            .iter()
            .copied()
            .find(|o| o.eq_ignore_ascii_case(code))
    };
    let base = wanted.split_once('-').map(|(base, _)| base);
    find(wanted.trim()).or_else(|| base.and_then(|b| find(b.trim())))
}

/// `wanted` mapped through [`offered_language`] in preference order, without duplicates;
/// languages the provider does not publish at all are dropped.
pub fn resolve_languages(wanted: &[String], offered: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for code in wanted.iter().filter_map(|w| offered_language(w, offered)) {
        if !out.iter().any(|o| o == code) {
            out.push(code.to_string());
        }
    }
    out
}

/// The codes to ask the registered provider `id` for: each wanted code as it publishes it
/// ([`offered_language`]), else unchanged, since a provider's `languages` list the editions it
/// is known for rather than every one it carries (MangaDex has Arabic without listing it).
pub fn languages_for(id: &str, wanted: &[String]) -> Vec<String> {
    let offered = registry::info(id)
        .map(|info| info.languages)
        .unwrap_or_default();
    let mut out: Vec<String> = Vec::new();
    for code in wanted
        .iter()
        .map(|w| offered_language(w, offered).unwrap_or(w))
    {
        if !out.iter().any(|o| o == code) {
            out.push(code.to_string());
        }
    }
    out
}

/// The code to ask provider `id` for when the user wants `language` (see [`languages_for`]).
pub fn language_for(id: &str, language: &str) -> String {
    let wanted = [language.to_string()];
    languages_for(id, &wanted)
        .into_iter()
        .next()
        .unwrap_or_else(|| language.to_string())
}
