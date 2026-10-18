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
