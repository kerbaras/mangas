//! A book over the Send-to-Kindle email limit is written as numbered parts, each small enough
//! to email on its own. Splitting is pure: page sizes in, page ranges out.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::domain::Result;
use crate::export::epub::{ChapterEntry, PageImage};
use crate::export::{azw3, free_names_in, Format, PageInfo};

/// Amazon's Send-to-Kindle email limit for one attached file, in bytes.
pub const SEND_TO_KINDLE_LIMIT: u64 = 50_000_000;
/// The part size offered by default: under the limit with room for what estimates miss.
pub const PART_SIZE: u64 = 45_000_000;
/// A written file over [`SEND_TO_KINDLE_LIMIT`] and its size on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Oversized {
    pub path: PathBuf,
    pub bytes: u64,
}

/// The files of `files` too big to email, measured on disk: what was really written, not an
/// estimate.
pub fn oversized(files: &[PathBuf]) -> Vec<Oversized> {
    let measure = |path: &PathBuf| {
        let bytes = std::fs::metadata(path).ok()?.len();
        (bytes > SEND_TO_KINDLE_LIMIT).then(|| Oversized {
            path: path.clone(),
            bytes,
        })
    };
    files.iter().filter_map(measure).collect()
}

/// Where part `out` is built while replacing: `.mangas-<pid>-<n>-<name>.part` next to it, unique
/// per part written, so two exports replacing the same book never share one.
pub(crate) fn staging(out: &Path) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let name = out.file_name().map(|n| n.to_string_lossy().into_owned());
    let pid = std::process::id();
    out.with_file_name(format!(
        ".mangas-{pid}-{n}-{}.part",
        name.unwrap_or_default()
    ))
}

/// What a page costs besides its JPEG (XHTML, manifest and zip entries).
const PAGE_OVERHEAD: u64 = 2_048;
/// What a part costs besides its pages (OPF, nav, styles).
const PART_OVERHEAD: u64 = 64_000;

/// A page JPEG's cost in a `format` book: AZW3 adds its standard record (at most 128 KiB).
pub fn page_cost(format: Format, jpeg: u64) -> u64 {
    let copy = (format == Format::Azw3).then(|| azw3::copy_bound(jpeg));
    jpeg + copy.unwrap_or(0) + PAGE_OVERHEAD
}

/// Consecutive page ranges, each `format` part (with its `cover` copy) at most `max` bytes. A
/// page that is bigger than a part on its own goes alone.
pub fn split_points(
    pages: &[PageInfo],
    cover: Option<&PageImage>,
    max: u64,
    format: Format,
) -> Vec<Range<usize>> {
    let fixed = PART_OVERHEAD + cover.map_or(0, |c| page_cost(format, c.jpeg.len() as u64));
    let budget = max.saturating_sub(fixed);
    let mut parts = Vec::new();
    let (mut start, mut used) = (0, 0u64);
    for (i, page) in pages.iter().enumerate() {
        let cost = page_cost(format, page.bytes as u64);
        if i > start && used + cost > budget {
            parts.push(start..i);
            (start, used) = (i, 0);
        }
        used += cost;
    }
    if start < pages.len() {
        parts.push(start..pages.len());
    }
    parts
}

/// The chapter entries of the pages in `range`, numbered from the part's first page. A part
/// that starts inside a chapter opens with that chapter's title.
pub fn chapters_in(chapters: &[ChapterEntry], range: &Range<usize>) -> Vec<ChapterEntry> {
    let open = chapters
        .iter()
        .rev()
        .find(|c| c.first_page <= range.start)
        .map(|c| ChapterEntry {
            title: c.title.clone(),
            first_page: 0,
        });
    let inside = chapters
        .iter()
        .filter(|c| c.first_page > range.start && c.first_page < range.end)
        .map(|c| ChapterEntry {
            title: c.title.clone(),
            first_page: c.first_page - range.start,
        });
    open.into_iter().chain(inside).collect()
}

/// `Title.epub` → `Title (part 2).epub`.
pub fn part_path(output: &Path, n: usize) -> PathBuf {
    let stem = output.file_stem().map(|s| s.to_string_lossy().into_owned());
    let name = format!("{} (part {n})", stem.unwrap_or_default());
    match output.extension() {
        Some(ext) => output.with_file_name(format!("{name}.{}", ext.to_string_lossy())),
        None => output.with_file_name(name),
    }
}

/// The book whose part `name` is named as by [`part_path`]: `Book (part 2).epub` → `Book.epub`.
pub fn part_owner(name: &str) -> Option<String> {
    let path = Path::new(name);
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let (book, n) = stem.strip_suffix(')')?.rsplit_once(" (part ")?;
    let numbered = !n.starts_with('0') && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit());
    let owner = match path.extension() {
        Some(ext) => format!("{book}.{}", ext.to_string_lossy()),
        None => book.to_owned(),
    };
    (numbered && !book.is_empty()).then_some(owner)
}

/// The book `part` is a part of (`Book (2) (part 1).epub` → `Book (2).epub`), else `part`.
pub fn part_book(part: &Path) -> PathBuf {
    let name = part.file_name().map(|n| n.to_string_lossy().into_owned());
    match name.as_deref().and_then(part_owner) {
        Some(book) => part.with_file_name(book),
        None => part.to_path_buf(),
    }
}

/// Where a book that replaces nothing is written: `output`, or its first name free in the folder
/// ([`free_names_in`]), so the book number never collides with an earlier set of parts.
pub fn free_book_path(output: &Path) -> Result<PathBuf> {
    let dir = output.parent().filter(|d| !d.as_os_str().is_empty());
    let name = output.file_name().map(|n| n.to_string_lossy().into_owned());
    let Some(name) = name else {
        return Ok(output.to_path_buf());
    };
    let free = free_names_in(dir.unwrap_or(Path::new(".")), &[name])?;
    Ok(free
        .first()
        .map_or(output.to_path_buf(), |n| output.with_file_name(n)))
}

/// The title of part `n` of `total`.
pub fn part_title(title: &str, n: usize, total: usize) -> String {
    format!("{title} (part {n} of {total})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_staged_part_gets_its_own_name_next_to_the_part() {
        let out = Path::new("/k/Big (part 2).epub");
        let (a, b) = (staging(out), staging(out));
        assert_ne!(a, b);
        for s in [a, b] {
            assert_eq!(s.parent(), out.parent());
            let name = s.file_name().unwrap().to_string_lossy().into_owned();
            assert!(name.starts_with(&format!(".mangas-{}-", std::process::id())));
            assert!(name.ends_with("-Big (part 2).epub.part"), "{name}");
        }
    }

    #[test]
    fn only_files_over_the_email_limit_on_disk_are_oversized() {
        let dir = tempfile::tempdir().unwrap();
        let [at, over] = ["at.epub", "over.epub"].map(|n| dir.path().join(n));
        std::fs::File::create(&at)
            .unwrap()
            .set_len(SEND_TO_KINDLE_LIMIT)
            .unwrap();
        let big = SEND_TO_KINDLE_LIMIT + 1;
        std::fs::File::create(&over).unwrap().set_len(big).unwrap();
        let gone = dir.path().join("gone.epub");
        let found = oversized(&[at, over.clone(), gone]);
        assert_eq!(
            found,
            vec![Oversized {
                path: over,
                bytes: big
            }]
        );
    }
}
