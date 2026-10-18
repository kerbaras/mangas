//! Book titles and file names from a template.

use super::Volume;
use crate::domain::{chapter_tag, season_tag, Chapter};

/// `{series}`, `{volume}`, `{first}` and `{last}` (chapter numbers) are replaced.
pub const DEFAULT_TEMPLATE: &str = "{series} v{volume}";

/// The book title for `volume` of `series`. A book without a volume number is named after its
/// chapters instead: `ch 12`, `ch 12-15`, or season tags such as `S1 Ep. 78 - S2 Ep. 1`.
pub fn render(template: &str, series: &str, volume: &Volume) -> String {
    let (first, last) = (volume.chapters.first(), volume.chapters.last());
    fn number(c: Option<&Chapter>) -> &str {
        c.map_or("", |c| c.number.trim())
    }
    let rendered = match &volume.number {
        None if template.contains("{volume}") => {
            format!("{} {}", series.trim(), chapters(first, last))
        }
        _ => template
            .replace("{series}", series.trim())
            .replace("{volume}", volume.number.as_deref().unwrap_or("").trim())
            .replace("{first}", number(first))
            .replace("{last}", number(last)),
    };
    collapse_spaces(&rendered)
}

/// `ch 12`, `ch 12-15`, `S2 Ep. 0` or `S1 Ep. 78 - S2 Ep. 1`.
fn chapters(first: Option<&Chapter>, last: Option<&Chapter>) -> String {
    let (Some(first), Some(last)) = (first, last) else {
        return String::new();
    };
    let (from, to) = (chapter_tag(first), chapter_tag(last));
    let seasons = [first, last]
        .iter()
        .any(|c| c.title.as_deref().and_then(season_tag).is_some());
    match () {
        _ if from == to => from,
        _ if seasons => format!("{from} - {to}"),
        _ => format!("ch {}-{}", first.number.trim(), last.number.trim()),
    }
}

fn collapse_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `title` as a portable `.epub` file name: path separators and characters Windows rejects
/// become spaces, trailing dots go, Windows device names get a suffix, and an empty result is
/// "Manga".
pub fn file_name(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let stem = collapse_spaces(&cleaned);
    let stem = stem.trim_end_matches(['.', ' ']);
    match stem {
        "" => "Manga.epub".into(),
        s if is_reserved(s) => format!("{s} (manga).epub"),
        s => format!("{s}.epub"),
    }
}

/// Whether Windows reserves `stem` as a device name (`CON`, `nul.x`, `COM1`…).
fn is_reserved(stem: &str) -> bool {
    let base = stem.split('.').next().unwrap_or(stem).trim_end();
    let upper = base.to_ascii_uppercase();
    let port = |prefix: &str| {
        upper
            .strip_prefix(prefix)
            .is_some_and(|n| matches!(n.as_bytes(), [b'1'..=b'9']))
    };
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL") || port("COM") || port("LPT")
}
