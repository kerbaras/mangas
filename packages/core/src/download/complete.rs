//! Whether a chapter folder holds the whole chapter. A finished download leaves a hidden marker
//! with its page count, so a folder that later lost pages (or a re-download that never
//! finished) reads as partial and gets its missing pages on a plain download.

use std::path::Path;

use super::cbz::is_image;

/// Hidden, so exports and imports (which skip dot files and non-images) never see it.
const MARKER: &str = ".mangas-pages";

/// Records that `dir` holds a whole chapter of `pages` pages.
pub(super) fn mark_complete(dir: &Path, pages: usize) {
    if let Err(e) = std::fs::write(dir.join(MARKER), format!("{pages}\n")) {
        log::warn!("could not record that {} is complete: {e}", dir.display());
    }
}

/// `dir` has every page its download recorded. A folder from before the marker existed counts
/// as complete once it has any page, as it always did.
pub(crate) fn is_complete(dir: &Path) -> bool {
    let recorded = std::fs::read_to_string(dir.join(MARKER))
        .ok()
        .and_then(|s| s.trim().parse().ok());
    holds_all(page_count(dir), recorded)
}

fn holds_all(on_disk: usize, recorded: Option<usize>) -> bool {
    match recorded {
        Some(pages) => on_disk >= pages && on_disk > 0,
        None => on_disk > 0,
    }
}

fn page_count(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .filter(|e| is_page(&e.file_name().to_string_lossy()))
                .count()
        })
        .unwrap_or(0)
}

fn is_page(name: &str) -> bool {
    !name.starts_with('.') && is_image(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recorded_count_must_be_met() {
        assert!(holds_all(29, Some(29)));
        assert!(!holds_all(24, Some(29)));
        assert!(!holds_all(0, Some(0)));
    }

    #[test]
    fn an_unmarked_folder_needs_one_page() {
        assert!(holds_all(1, None));
        assert!(!holds_all(0, None));
    }

    #[test]
    fn only_images_count_as_pages() {
        assert!(is_page("0001.jpg"));
        assert!(!is_page("0001.jpg.part"));
        assert!(!is_page(MARKER));
        assert!(!is_page("._0001.jpg"));
    }
}
