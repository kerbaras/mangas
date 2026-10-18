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

/// [`resolve_languages`] for the registered provider `id`. When it publishes none of `wanted`
/// (or is unknown), `wanted` comes back unchanged so the provider answers with an empty list.
pub fn languages_for(id: &str, wanted: &[String]) -> Vec<String> {
    let resolved = registry::info(id)
        .map(|info| resolve_languages(wanted, info.languages))
        .unwrap_or_default();
    if resolved.is_empty() {
        wanted.to_vec()
    } else {
        resolved
    }
}

/// The code to ask provider `id` for when the user wants `language` (see [`languages_for`]).
pub fn language_for(id: &str, language: &str) -> String {
    let wanted = [language.to_string()];
    languages_for(id, &wanted)
        .into_iter()
        .next()
        .unwrap_or_else(|| language.to_string())
}
