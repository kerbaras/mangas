//! Book titles and file names from a template.

use super::Volume;

/// `{series}`, `{volume}`, `{first}` and `{last}` (chapter numbers) are replaced.
pub const DEFAULT_TEMPLATE: &str = "{series} v{volume}";
/// Used instead of a template that names the volume when the book has no volume number.
const NO_VOLUME_TEMPLATE: &str = "{series} c{first}-{last}";
const SINGLE_CHAPTER_TEMPLATE: &str = "{series} c{first}";

/// The book title for `volume` of `series`.
pub fn render(template: &str, series: &str, volume: &Volume) -> String {
    let first = volume
        .chapters
        .first()
        .map(|c| c.number.trim())
        .unwrap_or("");
    let last = volume
        .chapters
        .last()
        .map(|c| c.number.trim())
        .unwrap_or("");
    let template = match &volume.number {
        None if template.contains("{volume}") && first == last => SINGLE_CHAPTER_TEMPLATE,
        None if template.contains("{volume}") => NO_VOLUME_TEMPLATE,
        _ => template,
    };
    let rendered = template
        .replace("{series}", series.trim())
        .replace("{volume}", volume.number.as_deref().unwrap_or("").trim())
        .replace("{first}", first)
        .replace("{last}", last);
    collapse_spaces(&rendered)
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
