//! What a `--replace` leaves of an earlier export of the same book (#134).

use std::path::{Path, PathBuf};

use super::parts::part_path;
use crate::domain::{Error, Result};

/// What a replacement of `output` leaves from an earlier export of the same book and format:
/// its parts numbered over `kept` (every part when the new book is one file), and the whole
/// `output` when the new book is split. Only names [`part_path`] gives `output` match.
pub fn stale_files(output: &Path, kept: usize) -> Result<Vec<PathBuf>> {
    let mut stale: Vec<PathBuf> = entries(output)?
        .into_iter()
        .filter(|p| part_number(output, p).is_some_and(|n| n > kept))
        .collect();
    if kept > 0 && output.is_file() {
        stale.push(output.to_path_buf());
    }
    stale.sort();
    Ok(stale)
}

/// Fails when `output`'s folder can't be listed, so a replacement stops before writing anything
/// it couldn't clean up after.
pub fn check_listable(output: &Path) -> Result<()> {
    entries(output).map(|_| ())
}

/// What `output`'s folder holds; a folder not made yet holds nothing.
fn entries(output: &Path) -> Result<Vec<PathBuf>> {
    let dir = output.parent().filter(|d| !d.as_os_str().is_empty());
    let dir = dir.unwrap_or(Path::new("."));
    match std::fs::read_dir(dir) {
        Ok(read) => Ok(read.flatten().map(|e| e.path()).collect()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(Error::Other(format!(
            "couldn't list {}: {e}",
            dir.display()
        ))),
    }
}

/// `n` when `path` is named `part_path(output, n)`.
fn part_number(output: &Path, path: &Path) -> Option<usize> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    let stem = output.file_stem()?.to_string_lossy().into_owned();
    let rest = name.strip_prefix(&stem)?.strip_prefix(" (part ")?;
    let n: usize = rest.split(')').next()?.parse().ok()?;
    let exact = part_path(output, n).file_name()? == path.file_name()?;
    (n > 0 && exact).then_some(n)
}

/// Removes [`stale_files`]; one already gone is fine.
pub fn remove_stale(output: &Path, kept: usize) -> Result<()> {
    for path in stale_files(output, kept)? {
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                let shown = path.display();
                return Err(Error::Other(format!(
                    "couldn't remove the old {shown}: {e}"
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_same_books_parts_over_the_new_count_are_stale() {
        let dir = tempfile::tempdir().unwrap();
        let names = [
            "Book.epub",
            "Book (part 1).epub",
            "Book (part 2).epub",
            "Book (part 3).epub",
            "Book (part 03).epub",
            "Book (part 3).kepub.epub",
            "Book (part 3).mobi",
            "Book (2) (part 3).epub",
            "Book (part 3) copy.epub",
            "Other.epub",
        ];
        for n in names {
            std::fs::write(dir.path().join(n), b"x").unwrap();
        }
        let out = dir.path().join("Book.epub");
        let named = |files: Vec<PathBuf>| -> Vec<String> {
            let name = |p: &PathBuf| p.file_name().unwrap().to_string_lossy().into_owned();
            files.iter().map(name).collect()
        };
        let split = named(stale_files(&out, 2).unwrap());
        assert_eq!(split, ["Book (part 3).epub", "Book.epub"]);
        let single = named(stale_files(&out, 0).unwrap());
        let parts = [
            "Book (part 1).epub",
            "Book (part 2).epub",
            "Book (part 3).epub",
        ];
        assert_eq!(single, parts);
        let new_folder = dir.path().join("not yet/Book.epub");
        assert!(stale_files(&new_folder, 0).unwrap().is_empty());
    }
}
