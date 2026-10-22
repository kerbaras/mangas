//! Shared by the MTP copy tests: a fault-injecting fake device and folder fixtures.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use mangas_core::send::mtp::{self, Entry, Fault, FolderStore, Handle, MtpStore, OnChunk};
use mangas_core::send::mtp::{Stopped, UploadFailed};

/// A [`FolderStore`] that fails one operation on cue and can't rename when `rename` is false.
pub struct Faulty {
    pub inner: FolderStore,
    pub fail_upload_after: Option<u64>,
    pub fail_rename: AtomicUsize,
    pub rename: bool,
    pub partial: bool,
}

pub fn faulty(root: &Path) -> Faulty {
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

pub fn book(dir: &Path, name: &str, len: usize) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, vec![7u8; len]).unwrap();
    path
}

pub fn names(dir: &Path) -> Vec<String> {
    let mut all: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    all.sort();
    all
}

pub fn run(
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

pub struct Fixture {
    pub _tmp: tempfile::TempDir,
    pub src: PathBuf,
    pub kindle: PathBuf,
}

pub fn fixture() -> Fixture {
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
