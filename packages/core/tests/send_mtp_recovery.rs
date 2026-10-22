//! What the next MTP copy does with the objects a copy cut off mid-swap left on the Kindle.

mod mtp_support;

use std::sync::atomic::{AtomicBool, AtomicUsize};

use mtp_support::{book, faulty, fixture, names, run, Faulty};

#[test]
fn a_swap_cut_off_before_the_rename_gets_the_old_book_back() {
    let f = fixture();
    let docs = f.kindle.join("documents");
    std::fs::create_dir(&docs).unwrap();
    book(&docs, ".mangas-77-3.old-Vol 1.pdf", 10);
    book(&docs, ".mangas-77-2.part", 4);
    let file = book(&f.src, "Vol 2.pdf", 20);
    run(&faulty(&f.kindle), &file, &AtomicBool::new(false)).unwrap();
    assert_eq!(names(&docs), ["Vol 1.pdf", "Vol 2.pdf"]);
    assert_eq!(std::fs::read(docs.join("Vol 1.pdf")).unwrap().len(), 10);
}

#[test]
fn a_swap_cut_off_after_the_rename_drops_only_the_old_copy() {
    let f = fixture();
    let docs = f.kindle.join("documents");
    std::fs::create_dir(&docs).unwrap();
    book(&docs, ".mangas-77-3.old-Vol 1.pdf", 10);
    book(&docs, "Vol 1.pdf", 30);
    let file = book(&f.src, "Vol 1.pdf", 30);
    assert!(
        run(&faulty(&f.kindle), &file, &AtomicBool::new(false))
            .unwrap()
            .already_there
    );
    let file = book(&f.src, "Vol 2.pdf", 20);
    run(&faulty(&f.kindle), &file, &AtomicBool::new(false)).unwrap();
    assert_eq!(names(&docs), ["Vol 1.pdf", "Vol 2.pdf"]);
    assert_eq!(std::fs::read(docs.join("Vol 1.pdf")).unwrap().len(), 30);
}

#[test]
fn an_aside_book_that_cannot_be_renamed_back_is_kept() {
    let f = fixture();
    let docs = f.kindle.join("documents");
    std::fs::create_dir(&docs).unwrap();
    book(&docs, ".mangas-77-3.old-Vol 1.pdf", 10);
    let store = Faulty {
        fail_rename: AtomicUsize::new(0),
        ..faulty(&f.kindle)
    };
    let file = book(&f.src, "Vol 2.pdf", 20);
    let _ = run(&store, &file, &AtomicBool::new(false));
    assert!(names(&docs).contains(&".mangas-77-3.old-Vol 1.pdf".to_string()));
}
