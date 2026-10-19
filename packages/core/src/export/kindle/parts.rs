//! A book over the Send-to-Kindle email limit is written as numbered parts, each small enough
//! to email on its own. Splitting is pure: page sizes in, page ranges out.

use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::export::epub::{ChapterEntry, PageImage};

/// Amazon's Send-to-Kindle email limit for one attached file, in bytes.
pub const SEND_TO_KINDLE_LIMIT: u64 = 50_000_000;
/// The part size offered by default: under the limit with room for what estimates miss.
pub const PART_SIZE: u64 = 45_000_000;
/// What a page costs besides its JPEG (XHTML, manifest and zip entries).
const PAGE_OVERHEAD: u64 = 2_048;
/// What a part costs besides its pages (OPF, nav, styles).
const PART_OVERHEAD: u64 = 64_000;

/// Consecutive page ranges, each part (with its `cover` copy) at most `max` bytes. A page that
/// is bigger than a part on its own goes alone.
pub fn split_points(pages: &[PageImage], cover: Option<&PageImage>, max: u64) -> Vec<Range<usize>> {
    let fixed = PART_OVERHEAD + cover.map_or(0, |c| c.jpeg.len() as u64 + PAGE_OVERHEAD);
    let budget = max.saturating_sub(fixed);
    let mut parts = Vec::new();
    let (mut start, mut used) = (0, 0u64);
    for (i, page) in pages.iter().enumerate() {
        let cost = page.jpeg.len() as u64 + PAGE_OVERHEAD;
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

/// The title of part `n` of `total`.
pub fn part_title(title: &str, n: usize, total: usize) -> String {
    format!("{title} (part {n} of {total})")
}
