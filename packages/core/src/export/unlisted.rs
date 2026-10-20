//! Free export names in a folder we may write to but not list (mode `0300`, a drop box): each
//! candidate is looked up by name instead, so the plan names the file the writer's `create_new`
//! claim gets (`Book (2).epub` when `Book.epub` is there).

use std::collections::HashSet;
use std::path::Path;

use super::kindle::part_path;
use super::output::candidate_names;

/// The names `wanted` get when written, in order, into unlistable `dir`: each takes its first
/// candidate that is neither in `dir` ([`held`]) nor planned for an earlier book.
pub(super) fn free_names(dir: &Path, wanted: &[String]) -> Vec<String> {
    let mut planned = HashSet::new();
    wanted
        .iter()
        .map(|name| {
            let free = candidate_names(name)
                .find(|c| !planned.contains(&c.to_lowercase()) && !held(dir, c))
                .unwrap_or_else(|| name.clone());
            planned.insert(free.to_lowercase());
            free
        })
        .collect()
}

/// `name`, or the first part of a book split under it (`Book (part 1).epub`), is in `dir`. A
/// lookup that fails (no search permission either) sees nothing, like the writer's claim.
fn held(dir: &Path, name: &str) -> bool {
    let book = dir.join(name);
    [part_path(&book, 1), book]
        .iter()
        .any(|p| p.symlink_metadata().is_ok())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn taken_names_and_split_books_are_skipped_without_listing() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("drop");
        std::fs::create_dir(&dir).unwrap();
        for f in ["Book.epub", "Other (part 1).epub"] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o300)).unwrap();
        let wanted = names(&["Book.epub", "Book.epub", "Other.epub", "New.epub"]);
        let free = free_names(&dir, &wanted);
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let want = [
            "Book (2).epub",
            "Book (3).epub",
            "Other (2).epub",
            "New.epub",
        ];
        assert_eq!(free, names(&want));
    }
}
