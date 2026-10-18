//! Pure helpers that turn folder and file names into chapter metadata.

use std::cmp::Ordering;

use sha2::{Digest, Sha256};

use crate::images::is_page_extension;

/// Chapter and volume numbers read from a name like `Vol.2 Ch.012.5 - Title`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedName {
    pub number: Option<String>,
    pub volume: Option<String>,
}

const CHAPTER_WORDS: [&str; 6] = ["chapter", "chap", "ch", "c", "episode", "ep"];
const VOLUME_WORDS: [&str; 3] = ["volume", "vol", "v"];

#[derive(Debug, PartialEq)]
enum Token {
    Word(String),
    Number(String),
    Hash,
}

fn tokens(name: &str) -> Vec<Token> {
    let chars: Vec<char> = name.to_lowercase().chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let start = i;
        if c.is_ascii_digit() {
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let has_fraction =
                i + 1 < chars.len() && chars[i] == '.' && chars[i + 1].is_ascii_digit();
            if has_fraction {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            out.push(Token::Number(chars[start..i].iter().collect()));
        } else if c.is_alphabetic() {
            while i < chars.len() && chars[i].is_alphabetic() {
                i += 1;
            }
            out.push(Token::Word(chars[start..i].iter().collect()));
        } else {
            if c == '#' {
                out.push(Token::Hash);
            }
            i += 1;
        }
    }
    out
}

/// "012.5" -> "12.5", "000" -> "0".
fn normalize(number: &str) -> String {
    let (int, frac) = number
        .split_once('.')
        .map_or((number, None), |(i, f)| (i, Some(f)));
    let int = int.trim_start_matches('0');
    let int = if int.is_empty() { "0" } else { int };
    frac.map_or(int.to_string(), |f| format!("{int}.{f}"))
}

fn is_word(token: Option<&Token>, words: &[&str]) -> bool {
    match token {
        Some(Token::Word(w)) => words.contains(&w.as_str()),
        Some(Token::Hash) => words.contains(&"#"),
        _ => false,
    }
}

/// Reads the chapter number from a keyword (`Chapter 12`, `ch.12`, `c012`, `#12`, `ep 3`), else
/// the last number in the name; `Vol 2` before the chapter fills the volume. A name with only a
/// volume uses it as the chapter number so volume folders keep their order.
pub fn parse_chapter_name(name: &str) -> ParsedName {
    let toks = tokens(strip_extension(name));
    let mut chapter_words = CHAPTER_WORDS.to_vec();
    chapter_words.push("#");
    let mut parsed = ParsedName::default();
    let mut volume_at = None;
    for (i, tok) in toks.iter().enumerate() {
        let Token::Number(n) = tok else { continue };
        let prev = i.checked_sub(1).and_then(|p| toks.get(p));
        if parsed.number.is_none() && is_word(prev, &chapter_words) {
            parsed.number = Some(normalize(n));
        } else if parsed.number.is_none() && parsed.volume.is_none() && is_word(prev, &VOLUME_WORDS)
        {
            parsed.volume = Some(normalize(n));
            volume_at = Some(i);
        }
    }
    if parsed.number.is_none() {
        parsed.number = toks
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, t)| match t {
                Token::Number(n) if Some(i) != volume_at => Some(normalize(n)),
                _ => None,
            })
            .or_else(|| parsed.volume.clone());
    }
    parsed
}

/// The name without a trailing `.cbz`/`.zip` (folder names keep their dots: `12.5` stays).
pub fn strip_extension(name: &str) -> &str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".cbz") || lower.ends_with(".zip") {
        &name[..name.len() - 4]
    } else {
        name
    }
}

/// ASCII slug (`One Piece!` -> `one-piece`), or a stable hash for names without ASCII alnum.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out: String = out.trim_end_matches('-').chars().take(64).collect();
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        format!("local-{}", short_hash(name))
    } else {
        out
    }
}

/// First 12 hex digits of SHA-256: stable ids for imported chapters.
pub fn short_hash(input: &str) -> String {
    Sha256::digest(input.as_bytes())
        .iter()
        .take(6)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A page the exporter can decode (see [`crate::images`]); hidden files never count.
pub fn is_page_image(name: &str) -> bool {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    !base.starts_with('.') && extension(base).is_some_and(|e| is_page_extension(&e))
}

pub fn extension(name: &str) -> Option<String> {
    let (stem, ext) = name.rsplit_once('.')?;
    (!stem.is_empty() && !ext.is_empty()).then(|| ext.to_ascii_lowercase())
}

/// Natural order: digit runs compare by value, so `page2` < `page10`; text ignores case.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a, b);
    loop {
        match (a.is_empty(), b.is_empty()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
        let (ra, rest_a) = run(a);
        let (rb, rest_b) = run(b);
        let ord = compare_runs(ra, rb);
        if ord != Ordering::Equal {
            return ord;
        }
        (a, b) = (rest_a, rest_b);
    }
}

fn run(s: &str) -> (&str, &str) {
    let digit = s.starts_with(|c: char| c.is_ascii_digit());
    let end = s
        .find(|c: char| c.is_ascii_digit() != digit)
        .unwrap_or(s.len());
    s.split_at(end)
}

fn compare_runs(a: &str, b: &str) -> Ordering {
    let both_digits =
        a.starts_with(|c: char| c.is_ascii_digit()) && b.starts_with(|c: char| c.is_ascii_digit());
    if both_digits {
        let (ta, tb) = (a.trim_start_matches('0'), b.trim_start_matches('0'));
        ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb))
    } else {
        a.to_lowercase().cmp(&b.to_lowercase())
    }
}
