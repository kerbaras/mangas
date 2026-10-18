//! Whether a chapter folder holds the whole chapter. Before fetching, a download records how
//! many pages the source has in a hidden marker, so a folder that later lost pages (or a run
//! that never finished) reads as partial and gets its missing pages on a plain download.

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::plan::present_indexes;
use crate::domain::Result;

/// Hidden, so exports and imports (which skip dot files and non-images) never see it.
const MARKER: &str = ".mangas-pages";

/// Records that the chapter in `dir` has `pages` pages, page `0001` to `pages`. Written aside
/// and renamed over the old marker, so a failed write never leaves it truncated and a
/// read-only old one is still replaced. Each write has its own aside name, so two writers never
/// rename each other's file away.
pub(super) fn mark_expected(dir: &Path, pages: usize) -> Result<()> {
    static WRITES: AtomicUsize = AtomicUsize::new(0);
    std::fs::create_dir_all(dir)?;
    let n = WRITES.fetch_add(1, Ordering::Relaxed);
    let fresh = dir.join(format!("{MARKER}.{}.{n}.new", std::process::id()));
    std::fs::write(&fresh, format!("{pages}\n"))?;
    std::fs::rename(fresh, dir.join(MARKER))?;
    Ok(())
}

/// `dir` has a page image for every index its marker records. A folder without a marker (from
/// before it existed) is unknown, so it is not complete: its next download fetches the page
/// list, writes the marker and fetches only the pages it lacks.
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
        None => false,
    }
}

/// An aside marker file a process died with before renaming it into place.
pub(super) fn is_marker_leftover(name: &str) -> bool {
    name.strip_prefix(MARKER)
        .is_some_and(|rest| rest.starts_with('.') && rest.ends_with(".new"))
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
        let names = file_names(dir.path());
        assert_eq!(
            names,
            HashSet::from([MARKER.to_string()]),
            "nothing left aside"
        );
    }

    #[test]
    fn concurrent_writers_each_land_their_marker() {
        let dir = tempfile::tempdir().unwrap();
        std::thread::scope(|s| {
            for pages in 1..=8 {
                let dir = dir.path();
                s.spawn(move || mark_expected(dir, pages).unwrap());
            }
        });
        assert_eq!(file_names(dir.path()), HashSet::from([MARKER.to_string()]));
    }

    #[test]
    fn an_unmarked_folder_is_never_complete() {
        assert!(!holds_all(&names(&["0001.jpg", "0002.jpg"]), None));
    }

    #[test]
    fn only_aside_marker_files_are_leftovers() {
        assert!(is_marker_leftover(".mangas-pages.4242.0.new"));
        assert!(!is_marker_leftover(MARKER));
        assert!(!is_marker_leftover(".mangas-pagesx.new"));
        assert!(!is_marker_leftover("0001.jpg"));
    }
}
