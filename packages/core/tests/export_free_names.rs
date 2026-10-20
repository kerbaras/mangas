use std::path::Path;

use mangas_core::export::kindle::{check_export_folder, part_book};
use mangas_core::export::{free_names_in, resolve_free_names};

fn names(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|x| x.to_string()).collect()
}

#[test]
fn free_names_keep_the_wanted_name_in_an_empty_folder() {
    let wanted = names(&["S v1.epub", "S v2.epub"]);
    assert_eq!(resolve_free_names(&[], &wanted), wanted);
}

#[test]
fn taken_names_get_the_next_free_suffix() {
    let existing = names(&["S v1.epub", "S v1 (2).epub", "S v2.epub", "S v1.pdf"]);
    let wanted = names(&["S v1.epub", "S v2.epub", "S v3.epub"]);
    assert_eq!(
        resolve_free_names(&existing, &wanted),
        names(&["S v1 (3).epub", "S v2 (2).epub", "S v3.epub"])
    );
}

#[test]
fn one_batch_never_gives_two_books_one_name() {
    let wanted = names(&["Book.cbz", "Book.cbz"]);
    assert_eq!(
        resolve_free_names(&[], &wanted),
        names(&["Book.cbz", "Book (2).cbz"])
    );
}

#[test]
fn folder_listing_matches_what_the_writer_claims() {
    let dir = tempfile::tempdir().unwrap();
    let wanted = names(&["S v1.epub"]);
    assert_eq!(
        free_names_in(&dir.path().join("missing"), &wanted).unwrap(),
        wanted
    );
    std::fs::write(dir.path().join("S v1.epub"), b"x").unwrap();
    assert_eq!(
        free_names_in(dir.path(), &wanted).unwrap(),
        names(&["S v1 (2).epub"])
    );
}

#[test]
fn names_differing_only_in_case_count_as_taken() {
    let existing = names(&["Book.epub"]);
    assert_eq!(
        resolve_free_names(&existing, &names(&["book.epub"])),
        names(&["book (2).epub"])
    );
}

#[test]
fn written_names_match_what_each_mode_writes() {
    use mangas_core::export::volume::{written_names, VolumePlan};
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("book.epub"), "old").unwrap();
    let plan = |name: &str| VolumePlan {
        number: None,
        title: name.into(),
        file_name: name.into(),
        chapters: Vec::new(),
    };
    let plans = [plan("Book"), plan("Book"), plan("Other")];
    assert_eq!(
        written_names(&plans, dir.path(), "epub", false).unwrap(),
        names(&["Book (2).epub", "Book (3).epub", "Other.epub"])
    );
    let numbered = [plan("Book"), plan("Book (2)")];
    assert_eq!(
        written_names(&numbered, dir.path(), "cbz", true).unwrap(),
        names(&["Book.cbz", "Book (2).cbz"])
    );
}

#[test]
fn an_earlier_set_of_parts_takes_its_book_name() {
    let existing = names(&[
        "Book (part 1).epub",
        "Book (part 2).epub",
        "Other (part 03).epub",
        "Other (part x).epub",
        "Third (part 1).mobi",
    ]);
    let wanted = names(&["Book.epub", "Other.epub", "Third.epub", "Book.epub"]);
    assert_eq!(
        resolve_free_names(&existing, &wanted),
        names(&["Book (2).epub", "Other.epub", "Third.epub", "Book (3).epub"])
    );
}

#[test]
fn a_part_names_its_book() {
    let book = |p: &str| part_book(Path::new(p)).display().to_string();
    assert_eq!(book("/o/Book (2) (part 1).epub"), "/o/Book (2).epub");
    assert_eq!(book("/o/Book (part 12).cbz"), "/o/Book.cbz");
    assert_eq!(book("/o/Book (part 03).epub"), "/o/Book (part 03).epub");
    assert_eq!(book("/o/Book.epub"), "/o/Book.epub");
}

#[cfg(unix)]
#[test]
fn only_a_split_or_a_replacement_must_list_the_folder() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("drop");
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o300)).unwrap();
    if std::fs::read_dir(&dir).is_ok() {
        return; // root ignores the mode; nothing to test
    }
    let check =
        |split, replace| check_export_folder(&dir, split, replace).map_err(|e| e.to_string());
    let results = [check(true, false), check(false, true), check(false, false)];
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let listed = format!("couldn't list {}: ", dir.display());
    assert!(results[0].as_ref().unwrap_err().starts_with(&listed));
    assert!(results[1].as_ref().unwrap_err().starts_with(&listed));
    assert_eq!(results[2], Ok(()));
    assert_eq!(
        check_export_folder(&tmp.path().join("new"), true, true).ok(),
        Some(())
    );
}
