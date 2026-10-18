//! Season-aware display labels for Webtoons-style episode titles.
//!
//! Sites that restart episode numbers every season keep an absolute index for ordering
//! ("episode 80") and put the season in the title ("[Season 2] Ep. 0"). [`season_label`]
//! rewrites such titles to a compact `S2 Ep. 0` prefix so lists and file names show the
//! number readers know, while the chapter number stays the absolute index.

const SEPARATORS: &[char] = &['-', ':', ',', '|', '–', '—'];

/// `"[Season 1] Ep. 1 - Headon's Floor"` → `"S1 Ep. 1 - Headon's Floor"`; `None` when the
/// title does not start with a season marker (`[Season N]`, `(Season N)`, `Season N`, `S2`).
pub fn season_label(title: &str) -> Option<String> {
    let (season, rest) = season(title.trim())?;
    let rest = rest.trim_start_matches(|c: char| c.is_whitespace() || SEPARATORS.contains(&c));
    let (episode, rest) = match episode(rest) {
        Some((n, rest)) => (format!(" Ep. {n}"), rest),
        None => (String::new(), rest),
    };
    Some(format!("S{season}{episode}{}", suffix(rest)))
}

fn season(s: &str) -> Option<(String, &str)> {
    let s = s.strip_prefix(['[', '(']).unwrap_or(s);
    let after = eat(s, "season")
        .map(str::trim_start)
        .or_else(|| eat(s, "s"))?;
    let (n, rest) = digits(after)?;
    let rest = rest.strip_prefix([']', ')']).unwrap_or(rest);
    (!rest.starts_with(char::is_alphanumeric)).then(|| (trim_zeros(n), rest))
}

fn episode(s: &str) -> Option<(String, &str)> {
    let after = eat(s, "episode").or_else(|| eat(s, "ep.").or_else(|| eat(s, "ep")))?;
    let (whole, rest) = digits(after.trim_start())?;
    let (fraction, rest) = match rest.strip_prefix('.').and_then(digits) {
        Some((f, rest)) => (format!(".{f}"), rest),
        None => (String::new(), rest),
    };
    (!rest.starts_with(char::is_alphanumeric)).then(|| (trim_zeros(whole) + &fraction, rest))
}

/// What follows the episode number, joined with " - " when the site used a separator.
fn suffix(rest: &str) -> String {
    let rest = rest.trim();
    let text = rest.trim_start_matches(SEPARATORS).trim_start();
    match (text.is_empty(), text.len() < rest.len()) {
        (true, _) => String::new(),
        (false, true) => format!(" - {text}"),
        (false, false) => format!(" {text}"),
    }
}

/// `s` without a case-insensitive ASCII `word` prefix.
fn eat<'a>(s: &'a str, word: &str) -> Option<&'a str> {
    let head = s.get(..word.len())?;
    head.eq_ignore_ascii_case(word).then(|| &s[word.len()..])
}

fn digits(s: &str) -> Option<(&str, &str)> {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    (end > 0).then(|| s.split_at(end))
}

fn trim_zeros(n: &str) -> String {
    let trimmed = n.trim_start_matches('0');
    if trimmed.is_empty() { "0" } else { trimmed }.to_string()
}
