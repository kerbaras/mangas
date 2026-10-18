//! Chapter languages: which ones a command asks for, and picking among translations.

use crate::domain::Chapter;
use crate::library::Settings;

/// `"es-la, EN pt-br"` -> `["es-la", "en", "pt-br"]`: trimmed, lowercase, first mention kept.
pub fn parse_languages(input: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for lang in input
        .split(|c: char| c == ',' || c.is_whitespace())
        .map(|l| l.trim().to_ascii_lowercase())
        .filter(|l| !l.is_empty())
    {
        if !out.contains(&lang) {
            out.push(lang);
        }
    }
    out
}

/// The languages given on the command line (`--lang es,en`), else the saved preferences.
pub fn requested_languages(flag: Option<&str>, settings: &Settings) -> Vec<String> {
    let given = flag.map(parse_languages).unwrap_or_default();
    if given.is_empty() {
        settings.languages()
    } else {
        given
    }
}

/// Chapters in one of `languages`, plus every downloaded chapter (owned files are never hidden).
pub fn in_languages(chapters: Vec<Chapter>, languages: &[String]) -> Vec<Chapter> {
    chapters
        .into_iter()
        .filter(|c| c.is_downloaded() || languages.contains(&c.language))
        .collect()
}

/// One translation per chapter: the most preferred language that has it. Translations match on
/// number and volume (a missing volume matches any, since sources omit it unevenly). Chapters
/// without a numeric number cannot be matched across languages and are all kept.
pub fn prefer_languages(chapters: &[Chapter], languages: &[String]) -> Vec<Chapter> {
    let rank = |c: &Chapter| {
        languages
            .iter()
            .position(|l| *l == c.language)
            .unwrap_or(usize::MAX)
    };
    chapters
        .iter()
        .filter(|c| {
            let Some(n) = c.number_value() else {
                return true;
            };
            !chapters
                .iter()
                .any(|o| o.number_value() == Some(n) && same_volume(o, c) && rank(o) < rank(c))
        })
        .cloned()
        .collect()
}

fn same_volume(a: &Chapter, b: &Chapter) -> bool {
    match (a.volume.as_deref(), b.volume.as_deref()) {
        (Some(x), Some(y)) => x.trim() == y.trim(),
        _ => true,
    }
}
