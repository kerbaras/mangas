//! Whether a chapter folder holds the whole chapter. Before fetching, a download records how
//! many pages the source has in a hidden marker, so a folder that later lost pages (or a run
//! that never finished) reads as partial and gets its missing pages on a plain download.

use std::collections::HashSet;
use std::path::Path;

use super::cbz::is_image;
use super::plan::present_indexes;
use crate::domain::Result;

/// Hidden, so exports and imports (which skip dot files and non-images) never see it.
const MARKER: &str = ".mangas-pages";

/// Records that the chapter in `dir` has `pages` pages, page `0001` to `pages`. Written aside
/// and renamed over the old marker, so a failed write never leaves it truncated and a
/// read-only old one is still replaced.
pub(super) fn mark_expected(dir: &Path, pages: usize) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let fresh = dir.join(format!("{MARKER}.new"));
    std::fs::write(&fresh, format!("{pages}\n"))?;
    std::fs::rename(fresh, dir.join(MARKER))?;
    Ok(())
}

/// `dir` has a page image for every index its marker records. A folder from before the marker
/// existed counts as complete once it has any page, as it always did.
pub(crate) fn is_complete(dir: &Path) -> bool {
    let recorded = std::fs::read_to_string(dir.join(MARKER))
        .ok()
        .and_then(|s| s.trim().parse().ok());
    holds_all(&file_names(dir), recorded)
}

fn holds_all(names: &HashSet<String>, recorded: Option<usize>) -> bool {
    match recorded {
        Some(pages) => {
            let present = present_indexes(names);
            pages > 0 && (1..=pages).all(|i| present.contains(&i))
        }
        None => names.iter().any(|n| !n.starts_with('.') && is_image(n)),
    }
}

fn file_names(dir: &Path) -> HashSet<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> HashSet<String> {
        list.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn every_recorded_index_needs_a_page() {
        assert!(holds_all(&names(&["0001.jpg", "0002.png"]), Some(2)));
        assert!(!holds_all(&names(&["0001.jpg"]), Some(2)));
        assert!(!holds_all(&names(&[]), Some(0)));
    }

    #[test]
    fn extra_images_do_not_stand_in_for_a_missing_page() {
        let dir = names(&[
            "0001.jpg",
            "0001.png",
            "0003.jpg",
            "cover.jpg",
            "0002.jpg.part",
        ]);
        assert!(!holds_all(&dir, Some(3)));
    }

    #[test]
    fn a_read_only_marker_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        mark_expected(dir.path(), 6).unwrap();
        let marker = dir.path().join(MARKER);
        let mut perms = std::fs::metadata(&marker).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&marker, perms).unwrap();
        mark_expected(dir.path(), 8).unwrap();
        assert_eq!(std::fs::read_to_string(marker).unwrap(), "8\n");
        assert!(!dir.path().join(format!("{MARKER}.new")).exists());
    }

    #[test]
    fn an_unmarked_folder_needs_one_page() {
        assert!(holds_all(&names(&["0001.jpg"]), None));
        assert!(!holds_all(&names(&["0001.jpg.part", MARKER]), None));
    }
}
