//! A folder that plays an MTP storage: QA and tests run the real copy protocol (`transfer`)
//! against it on machines without a Kindle. Like the `mtp-rs` USB backend, a stopped upload
//! leaves its partial object behind for the caller to delete.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use super::store::{Entry, Fault, Handle, MtpStore, OnChunk, UploadFailed};

const CHUNK: usize = 256 << 10;

/// One storage backed by `root`; `pause` is slept per chunk so QA can watch and cancel.
pub struct FolderStore {
    name: String,
    root: PathBuf,
    pause: Duration,
    handles: Mutex<Vec<PathBuf>>,
}

impl FolderStore {
    pub fn new(name: &str, root: &Path, pause: Duration) -> FolderStore {
        FolderStore {
            name: name.to_string(),
            root: root.to_path_buf(),
            pause,
            handles: Mutex::new(Vec::new()),
        }
    }

    fn handle(&self, path: PathBuf) -> Handle {
        let mut handles = self.handles.lock().unwrap_or_else(|e| e.into_inner());
        let at = match handles.iter().position(|p| *p == path) {
            Some(at) => at,
            None => {
                handles.push(path);
                handles.len() - 1
            }
        };
        at as Handle + 1
    }

    fn path(&self, handle: Option<Handle>) -> Result<PathBuf, Fault> {
        let Some(handle) = handle else {
            return Ok(self.root.clone());
        };
        let handles = self.handles.lock().unwrap_or_else(|e| e.into_inner());
        let at = (handle as usize).checked_sub(1);
        at.and_then(|at| handles.get(at).cloned())
            .ok_or_else(|| Fault::Other(format!("no object {handle}")))
    }
}

fn fault(e: std::io::Error) -> Fault {
    match e.kind() {
        std::io::ErrorKind::StorageFull => Fault::Full,
        std::io::ErrorKind::PermissionDenied => Fault::Denied,
        std::io::ErrorKind::NotFound => Fault::Disconnected,
        _ => Fault::Other(e.to_string()),
    }
}

#[async_trait::async_trait]
impl MtpStore for FolderStore {
    fn name(&self) -> &str {
        &self.name
    }

    fn can_rename(&self) -> bool {
        true
    }

    async fn list(&self, parent: Option<Handle>) -> Result<Vec<Entry>, Fault> {
        let mut out = Vec::new();
        for item in std::fs::read_dir(self.path(parent)?).map_err(fault)? {
            let item = item.map_err(fault)?;
            let meta = item.metadata().map_err(fault)?;
            out.push(Entry {
                handle: self.handle(item.path()),
                name: item.file_name().to_string_lossy().into_owned(),
                size: meta.len(),
                folder: meta.is_dir(),
            });
        }
        Ok(out)
    }

    async fn create_folder(&self, parent: Option<Handle>, name: &str) -> Result<Handle, Fault> {
        let path = self.path(parent)?.join(name);
        std::fs::create_dir(&path).map_err(fault)?;
        Ok(self.handle(path))
    }

    async fn upload(
        &self,
        parent: Handle,
        name: &str,
        file: &Path,
        _size: u64,
        on_chunk: OnChunk<'_>,
    ) -> Result<Handle, UploadFailed> {
        let failed = |fault, partial| UploadFailed { fault, partial };
        let path = self
            .path(Some(parent))
            .map_err(|f| failed(f, None))?
            .join(name);
        let mut src = File::open(file).map_err(|e| failed(fault(e), None))?;
        let mut dst = File::create_new(&path).map_err(|e| failed(fault(e), None))?;
        let partial = Some(self.handle(path));
        let mut buf = vec![0u8; CHUNK];
        let mut done = 0u64;
        loop {
            let n = src.read(&mut buf).map_err(|e| failed(fault(e), partial))?;
            if n == 0 {
                break;
            }
            dst.write_all(&buf[..n])
                .map_err(|e| failed(fault(e), partial))?;
            done += n as u64;
            std::thread::sleep(self.pause);
            if on_chunk(done).is_break() {
                return Err(failed(Fault::Cancelled, partial));
            }
        }
        dst.sync_all().map_err(|e| failed(fault(e), partial))?;
        partial.ok_or_else(|| failed(Fault::Other("no handle".into()), None))
    }

    async fn delete(&self, handle: Handle) -> Result<(), Fault> {
        let path = self.path(Some(handle))?;
        match path.is_dir() {
            true => std::fs::remove_dir(&path),
            false => std::fs::remove_file(&path),
        }
        .map_err(fault)
    }

    async fn rename(&self, handle: Handle, name: &str) -> Result<(), Fault> {
        let from = self.path(Some(handle))?;
        let to = from.with_file_name(name);
        if to.exists() {
            return Err(Fault::Other(format!("{name} exists")));
        }
        std::fs::rename(&from, &to).map_err(fault)?;
        let mut handles = self.handles.lock().unwrap_or_else(|e| e.into_inner());
        let at = (handle as usize).checked_sub(1);
        if let Some(slot) = at.and_then(|i| handles.get_mut(i)) {
            *slot = to;
        }
        Ok(())
    }
}
