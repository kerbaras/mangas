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
/// become spaces, trailing dots go, and an empty result is "Manga".
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
    let stem = if stem.is_empty() { "Manga" } else { stem };
    format!("{stem}.epub")
}
