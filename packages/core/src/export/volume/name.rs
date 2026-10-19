//! Book titles and file names from a template.

use std::path::Path;

use super::Volume;
use crate::domain::{chapter_tag, season_tag, Chapter};
use crate::export::Format;

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

/// Longest file stem, in bytes. With a ` (9999)` suffix, the extension and the writer's `.part`
/// the name stays well under the 255-byte (or 255 UTF-16 unit) limit of common file systems.
pub const MAX_STEM_BYTES: usize = 200;

/// Marks where a name too long for the file system was shortened.
const ELLIPSIS: &str = "…";

/// `title` as a portable `.epub` file name: path separators and characters Windows rejects
/// become spaces, trailing dots go, Windows device names get a suffix, an empty result is
/// "Manga", and a stem over [`MAX_STEM_BYTES`] is cut at a character boundary, ending in "…".
pub fn file_name(title: &str) -> String {
    series_file_name(title, "")
}

/// [`file_name`] for a title rendered from `series`: when it is too long the series part is
/// shortened first, so the volume or chapter range (`v3`, `ch 1-5`) stays in the name.
pub fn series_file_name(title: &str, series: &str) -> String {
    let stem = portable(title);
    let stem = match stem.len() <= MAX_STEM_BYTES {
        true => stem,
        false => shorten(&stem, &portable(series)),
    };
    let stem = stem.trim_end_matches(['.', ' ']);
    match stem {
        "" => "Manga.epub".into(),
        s if is_reserved(s) => format!("{s} (manga).epub"),
        s => format!("{s}.epub"),
    }
}

/// [`file_name`] with the extension of `format`, as the export writes it.
pub fn file_name_as(title: &str, format: Format) -> String {
    let name = file_name(title);
    let path = Path::new(&name).with_extension(format.id());
    path.to_string_lossy().into_owned()
}

fn portable(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    collapse_spaces(&cleaned)
}

/// `stem` cut to [`MAX_STEM_BYTES`]: inside its first `series`, else at the end.
fn shorten(stem: &str, series: &str) -> String {
    let at = stem.find(series).filter(|_| !series.is_empty());
    let rest = stem.len() - series.len();
    if let Some(at) = at.filter(|_| rest + ELLIPSIS.len() < MAX_STEM_BYTES) {
        let kept = prefix(series, MAX_STEM_BYTES - rest - ELLIPSIS.len()).trim_end();
        let (before, after) = (&stem[..at], &stem[at + series.len()..]);
        return format!("{before}{kept}{ELLIPSIS}{after}");
    }
    let kept = prefix(stem, MAX_STEM_BYTES - ELLIPSIS.len()).trim_end();
    format!("{kept}{ELLIPSIS}")
}

/// The longest start of `s` of at most `max` bytes that ends on a character boundary.
fn prefix(s: &str, max: usize) -> &str {
    let mut end = max.min(s.len());
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
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
