//! The MTP copy protocol against a folder that plays the Kindle's storage, with faults
//! injected: `.part` upload, replace, cancel mid-upload, failed upload, Already on Kindle.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use mangas_core::send::mtp::{self, Entry, Fault, FolderStore, Handle, MtpStore, OnChunk};
use mangas_core::send::mtp::{Stopped, UploadFailed};

/// A [`FolderStore`] that fails one operation on cue and can't rename when `rename` is false.
struct Faulty {
    inner: FolderStore,
    fail_upload_after: Option<u64>,
    fail_rename: AtomicUsize,
    rename: bool,
    partial: bool,
}

fn faulty(root: &Path) -> Faulty {
    Faulty {
        inner: FolderStore::new("Kindle Paperwhite", root, Duration::ZERO),
        fail_upload_after: None,
        fail_rename: AtomicUsize::new(usize::MAX),
        rename: true,
        partial: true,
    }
}

#[async_trait::async_trait]
impl MtpStore for Faulty {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn can_rename(&self) -> bool {
        self.rename
    }
    async fn list(&self, parent: Option<Handle>) -> Result<Vec<Entry>, Fault> {
        self.inner.list(parent).await
    }
    async fn create_folder(&self, parent: Option<Handle>, name: &str) -> Result<Handle, Fault> {
        self.inner.create_folder(parent, name).await
    }
    async fn upload(
        &self,
        parent: Handle,
        name: &str,
        file: &Path,
        size: u64,
        on_chunk: OnChunk<'_>,
    ) -> Result<Handle, UploadFailed> {
        let limit = self.fail_upload_after;
        let mut cut = |done: u64| match limit {
            Some(limit) if done > limit => std::ops::ControlFlow::Break(()),
            _ => on_chunk(done),
        };
        let up = self.inner.upload(parent, name, file, size, &mut cut).await;
        match (up, limit) {
            (Err(mut e), Some(_)) if e.fault == Fault::Cancelled => {
                e.fault = Fault::Disconnected;
                // A WPD device commits nothing and reports no partial handle.
                e.partial = e.partial.filter(|_| self.partial);
                Err(e)
            }
            (up, _) => up,
        }
    }
    async fn delete(&self, handle: Handle) -> Result<(), Fault> {
        self.inner.delete(handle).await
    }
    async fn rename(&self, handle: Handle, name: &str) -> Result<(), Fault> {
        if self.fail_rename.fetch_sub(1, Ordering::SeqCst) == 0 {
            return Err(Fault::Other("rename refused".into()));
        }
        self.inner.rename(handle, name).await
    }
}

fn book(dir: &Path, name: &str, len: usize) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, vec![7u8; len]).unwrap();
    path
}

fn names(dir: &Path) -> Vec<String> {
    let mut all: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    all.sort();
    all
}

fn run(
    store: &dyn MtpStore,
    file: &Path,
    cancel: &AtomicBool,
) -> Result<mangas_core::send::usb::Copied, Stopped> {
    let size = std::fs::metadata(file).unwrap().len();
    let name = file.file_name().unwrap().to_string_lossy().into_owned();
    let cancelled = || cancel.load(Ordering::SeqCst);
    let mut progress = |done, total| {
        assert!(done <= total);
        if done >= 300_000 && name.contains("cancel") {
            cancel.store(true, Ordering::SeqCst);
        }
    };
    futures::executor::block_on(mtp::send(
        store,
        file,
        &name,
        size,
        &cancelled,
        &mut progress,
    ))
}

struct Fixture {
    _tmp: tempfile::TempDir,
    src: PathBuf,
    kindle: PathBuf,
}

fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let (src, kindle) = (tmp.path().join("src"), tmp.path().join("kindle"));
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&kindle).unwrap();
    Fixture {
        _tmp: tmp,
        src,
        kindle,
    }
}

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
