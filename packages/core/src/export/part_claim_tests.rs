use std::io::Write;

use crate::export::output::write_file;

fn fill(text: &'static str) -> impl FnOnce(std::fs::File) -> crate::Result<()> {
    move |mut f| {
        f.write_all(text.as_bytes())
            .map_err(crate::domain::unplaced)
    }
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
        f.write_all(b"half").map_err(crate::domain::unplaced)?;
        Err(crate::Error::Other("stopped".into()))
    });
    assert!(failed.is_err());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn a_part_still_linked_to_its_published_book_is_not_emptied() {
    let dir = tempfile::tempdir().unwrap();
    let book = dir.path().join("Book.epub");
    std::fs::write(&book, b"published").unwrap();
    std::fs::hard_link(&book, dir.path().join("Book.epub.part")).unwrap();
    let part = super::claim_part(&book)
        .unwrap()
        .expect("a leftover part is taken over");
    assert_eq!(part.metadata().unwrap().len(), 0);
    assert_eq!(std::fs::read(&book).unwrap(), b"published");
}

#[test]
fn publishing_never_replaces_a_book_that_appeared_meanwhile() {
    let dir = tempfile::tempdir().unwrap();
    let book = dir.path().join("Book.epub");
    let claim = super::claim_free_name(&book).unwrap();
    std::fs::write(&book, b"someone else's").unwrap();
    let err = super::publish(&claim).unwrap_err();
    let crate::Error::Io { path, source } = &err else {
        panic!("{err:?}")
    };
    assert_eq!(
        (path, source.kind()),
        (&book, std::io::ErrorKind::AlreadyExists)
    );
    assert_eq!(err.to_string().matches("Book.epub").count(), 1, "{err}");
    assert_eq!(std::fs::read(&book).unwrap(), b"someone else's");
}
