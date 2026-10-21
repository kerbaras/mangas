//! Claiming a free export name through its `.part` file (#301): the final name only ever
//! appears by renaming a finished part, so an interrupted export leaves at most `<name>.part`,
//! never an empty or half-written book.

use std::ffi::OsString;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::output::{candidate_names, file_names, folded};

/// A free name and its part, held locked for the whole write.
pub(super) struct Claim {
    pub target: PathBuf,
    pub part: PathBuf,
    pub file: File,
}

/// `<file name>.part` next to `target`; appended to the whole name so `book` and `book.epub`
/// never share one temporary file.
pub(super) fn part_file(target: &Path) -> PathBuf {
    let mut name = target.file_name().map(OsString::from).unwrap_or_default();
    name.push(".part");
    target.with_file_name(name)
}

/// The first of `path`, `stem (2).ext`, ... with no file and no part another export is writing
/// (it holds the part locked). A part left by an interrupted export is taken over, emptied.
/// Names already in the folder in another case count as taken, so case-insensitive disks match
/// the preview.
pub(super) fn claim_free_name(path: &Path) -> std::io::Result<Claim> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty());
    let taken = folded(&file_names(dir.unwrap_or(Path::new(".")))?);
    let free = candidate_names(&name).filter(|c| !taken.contains(&c.to_lowercase()));
    for target in free.map(|n| path.with_file_name(n)) {
        if exists(&target)? {
            continue;
        }
        let Some(file) = claim_part(&target)? else {
            continue;
        };
        if exists(&target)? {
            continue;
        }
        let part = part_file(&target);
        return Ok(Claim { target, part, file });
    }
    unreachable!("candidate_names is infinite")
}

/// `target`'s `.part`, emptied and locked, or `None` when another export holds it. A part left by
/// an interrupted export is taken over.
pub(super) fn claim_part(target: &Path) -> std::io::Result<Option<File>> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(part_file(target))?;
    if !lock(&file)? {
        return Ok(None);
    }
    file.set_len(0)?;
    Ok(Some(file))
}

/// Moves the finished part onto its name, unless a file appeared there meanwhile.
pub(super) fn publish(claim: &Claim) -> std::io::Result<()> {
    if exists(&claim.target)? {
        return Err(std::io::Error::new(
            ErrorKind::AlreadyExists,
            format!("{} appeared while exporting", claim.target.display()),
        ));
    }
    std::fs::rename(&claim.part, &claim.target)
}

/// Whether we now hold the part's lock: false when another export holds it. A file system with
/// no locks (some network shares) counts as ours, as before locks were used.
fn lock(file: &File) -> std::io::Result<bool> {
    match file.try_lock() {
        Ok(()) => Ok(true),
        Err(TryLockError::WouldBlock) => Ok(false),
        Err(TryLockError::Error(e)) if e.kind() == ErrorKind::Unsupported => Ok(true),
        Err(TryLockError::Error(e)) => Err(e),
    }
}

fn exists(path: &Path) -> std::io::Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use crate::export::output::write_file;

    fn fill(text: &'static str) -> impl FnOnce(std::fs::File) -> crate::Result<()> {
        move |mut f| Ok(f.write_all(text.as_bytes())?)
    }

    #[test]
    fn a_part_left_by_a_killed_export_is_taken_over() {
        let dir = tempfile::tempdir().unwrap();
        let book = dir.path().join("Book.epub");
        std::fs::write(dir.path().join("Book.epub.part"), b"stale and longer").unwrap();
        assert_eq!(write_file(&book, false, fill("new")).unwrap(), book);
        assert_eq!(std::fs::read(&book).unwrap(), b"new");
        assert!(!dir.path().join("Book.epub.part").exists());
    }

    #[test]
    fn a_part_another_export_is_writing_moves_this_one_to_the_next_name() {
        let dir = tempfile::tempdir().unwrap();
        let busy = std::fs::File::create(dir.path().join("Book.epub.part")).unwrap();
        busy.try_lock().unwrap();
        let written = write_file(&dir.path().join("Book.epub"), false, fill("b")).unwrap();
        assert_eq!(written, dir.path().join("Book (2).epub"));
        assert!(!dir.path().join("Book.epub").exists());
    }

    #[test]
    fn a_replacement_takes_over_the_part_left_by_a_killed_export() {
        let dir = tempfile::tempdir().unwrap();
        let book = dir.path().join("Book.epub");
        std::fs::write(&book, b"old").unwrap();
        std::fs::write(dir.path().join("Book.epub.part"), b"stale and longer").unwrap();
        assert_eq!(write_file(&book, true, fill("new")).unwrap(), book);
        assert_eq!(std::fs::read(&book).unwrap(), b"new");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn a_failed_write_leaves_neither_the_book_nor_its_part() {
        let dir = tempfile::tempdir().unwrap();
        let book = dir.path().join("Book.epub");
        let failed = write_file(&book, false, |mut f| {
            f.write_all(b"half")?;
            Err(crate::Error::Other("stopped".into()))
        });
        assert!(failed.is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
