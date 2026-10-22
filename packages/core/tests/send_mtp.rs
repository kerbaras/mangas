//! The MTP copy protocol against a folder that plays the Kindle's storage, with faults
//! injected: `.part` upload, replace, cancel mid-upload, failed upload, Already on Kindle.

mod mtp_support;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use mangas_core::send::mtp::{Fault, Stopped};

use mtp_support::{book, faulty, fixture, names, run};

#[test]
fn copies_into_documents_creating_it_and_reports_the_device_path() {
    let f = fixture();
    let file = book(&f.src, "Vol 1.azw3", 600_000);
    let copied = run(&faulty(&f.kindle), &file, &AtomicBool::new(false)).unwrap();
    assert_eq!(copied.path, Path::new("documents/Vol 1.azw3"));
    assert!(!copied.already_there);
    assert_eq!(names(&f.kindle.join("documents")), ["Vol 1.azw3"]);
    assert_eq!(
        std::fs::read(f.kindle.join("documents/Vol 1.azw3"))
            .unwrap()
            .len(),
        600_000
    );
}

#[test]
fn same_name_and_size_is_already_on_kindle_and_uploads_nothing() {
    let f = fixture();
    std::fs::create_dir(f.kindle.join("Documents")).unwrap();
    book(&f.kindle.join("Documents"), "Vol 1.pdf", 1000);
    let file = book(&f.src, "Vol 1.pdf", 1000);
    let mut store = faulty(&f.kindle);
    store.fail_upload_after = Some(0);
    let copied = run(&store, &file, &AtomicBool::new(false)).unwrap();
    assert!(copied.already_there);
    assert_eq!(names(&f.kindle), ["Documents"]);
}

#[test]
fn a_different_size_replaces_the_old_book_and_leaves_no_scratch_objects() {
    let f = fixture();
    std::fs::create_dir(f.kindle.join("documents")).unwrap();
    book(&f.kindle.join("documents"), "Vol 1.pdf", 10);
    book(&f.kindle.join("documents"), ".mangas-1-0.part", 5);
    let file = book(&f.src, "Vol 1.pdf", 2000);
    let copied = run(&faulty(&f.kindle), &file, &AtomicBool::new(false)).unwrap();
    assert!(!copied.already_there);
    assert_eq!(names(&f.kindle.join("documents")), ["Vol 1.pdf"]);
    assert_eq!(
        std::fs::read(f.kindle.join("documents/Vol 1.pdf"))
            .unwrap()
            .len(),
        2000
    );
}

#[test]
fn cancel_mid_upload_deletes_the_part_and_keeps_the_old_book() {
    let f = fixture();
    std::fs::create_dir(f.kindle.join("documents")).unwrap();
    book(&f.kindle.join("documents"), "cancel me.pdf", 10);
    let file = book(&f.src, "cancel me.pdf", 2_000_000);
    let cancel = AtomicBool::new(false);
    let err = run(&faulty(&f.kindle), &file, &cancel).unwrap_err();
    assert_eq!(err, Stopped(Fault::Cancelled));
    assert_eq!(names(&f.kindle.join("documents")), ["cancel me.pdf"]);
    assert_eq!(
        std::fs::read(f.kindle.join("documents/cancel me.pdf"))
            .unwrap()
            .len(),
        10
    );
}

#[test]
fn a_failed_upload_deletes_the_partial_object_with_or_without_its_handle() {
    for partial in [true, false] {
        let f = fixture();
        std::fs::create_dir(f.kindle.join("documents")).unwrap();
        book(&f.kindle.join("documents"), "Vol 2.azw3", 10);
        let file = book(&f.src, "Vol 2.azw3", 1_000_000);
        let mut store = faulty(&f.kindle);
        store.fail_upload_after = Some(300_000);
        store.partial = partial;
        let err = run(&store, &file, &AtomicBool::new(false)).unwrap_err();
        assert_eq!(err, Stopped(Fault::Disconnected));
        assert_eq!(
            names(&f.kindle.join("documents")),
            ["Vol 2.azw3"],
            "partial {partial}"
        );
    }
}

#[test]
fn a_refused_rename_puts_the_old_book_back() {
    for at in [0, 1] {
        let f = fixture();
        std::fs::create_dir(f.kindle.join("documents")).unwrap();
        book(&f.kindle.join("documents"), "Vol 3.pdf", 10);
        let file = book(&f.src, "Vol 3.pdf", 5000);
        let store = faulty(&f.kindle);
        store.fail_rename.store(at, Ordering::SeqCst);
        let err = run(&store, &file, &AtomicBool::new(false)).unwrap_err();
        assert!(matches!(err, Stopped(Fault::Other(_))), "{err:?}");
        assert_eq!(
            names(&f.kindle.join("documents")),
            ["Vol 3.pdf"],
            "rename {at}"
        );
        assert_eq!(
            std::fs::read(f.kindle.join("documents/Vol 3.pdf"))
                .unwrap()
                .len(),
            10
        );
    }
}

#[test]
fn without_rename_the_old_book_is_replaced_in_place() {
    let f = fixture();
    std::fs::create_dir(f.kindle.join("documents")).unwrap();
    book(&f.kindle.join("documents"), "Vol 4.pdf", 10);
    let file = book(&f.src, "Vol 4.pdf", 5000);
    let mut store = faulty(&f.kindle);
    store.rename = false;
    run(&store, &file, &AtomicBool::new(false)).unwrap();
    assert_eq!(names(&f.kindle.join("documents")), ["Vol 4.pdf"]);
    assert_eq!(
        std::fs::read(f.kindle.join("documents/Vol 4.pdf"))
            .unwrap()
            .len(),
        5000
    );
}
