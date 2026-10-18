//! Chapter languages: which ones a command asks for, and picking among translations.

use crate::domain::{Chapter, Error, Result};
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

/// [`parse_languages`] for a list to save: every code must look like `en`, `es-la`, `pt-br` or
/// `es-419` (`^[a-z]{2,3}(-([a-z]{2,4}|[0-9]{3}))?$`, lowercase), and at least one is required.
pub fn validate_languages(input: &str) -> Result<Vec<String>> {
    let codes = input.split(|c: char| c == ',' || c.is_whitespace());
    if let Some(bad) = codes.filter(|c| !c.is_empty()).find(|c| !well_formed(c)) {
        return Err(Error::Other(format!(
            "'{bad}' is not a language code; use forms like en, es-la or pt-br"
        )));
    }
    let parsed = parse_languages(input);
    match parsed.is_empty() {
        true => Err(Error::Other(format!("no language in '{input}'"))),
        false => Ok(parsed),
    }
}

fn well_formed(code: &str) -> bool {
    let letters = |s: &str, min: usize, max: usize| {
        (min..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_lowercase())
    };
    let digits = |s: &str| s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit());
    match code.split_once('-') {
        Some((base, region)) => letters(base, 2, 3) && (letters(region, 2, 4) || digits(region)),
        None => letters(code, 2, 3),
    }
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
