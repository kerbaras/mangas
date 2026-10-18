//! A lease on the models folder shared by every Mangas process (app and CLI). A detector
//! download, a data-folder move and a removal each hold it, so none of them changes the folder
//! under another. The holder rewrites the file every few seconds; a file left alone for
//! [`STALE`] belongs to a process that died, and the next caller takes it over.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::domain::{Error, Result};

pub const LEASE_FILE: &str = ".detectors.lease";
pub const STALE: Duration = Duration::from_secs(30);
const BEAT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    Download,
    Move,
    Remove,
}

#[derive(Serialize, Deserialize)]
struct Holder {
    pid: u32,
    purpose: Purpose,
}

/// Held while the models folder is in use; dropping it releases the folder.
pub struct Lease {
    path: PathBuf,
    stop: Arc<AtomicBool>,
    beat: Option<JoinHandle<()>>,
}

pub fn lease_path(models_dir: &Path) -> PathBuf {
    models_dir.join(LEASE_FILE)
}

/// Takes the lease, or fails with a message naming what holds it.
pub fn acquire(models_dir: &Path, purpose: Purpose) -> Result<Lease> {
    fs::create_dir_all(models_dir)?;
    let path = lease_path(models_dir);
    let holder = Holder {
        pid: std::process::id(),
        purpose,
    };
    let record = serde_json::to_vec(&holder).map_err(|e| Error::Other(e.to_string()))?;
    for _ in 0..2 {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(&record)?;
                return Ok(start(path, record));
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                if let Some(busy) = busy(&path) {
                    return Err(Error::Other(busy));
                }
                remove_if_present(&path)?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    Err(Error::Other("the models folder is busy; try again".into()))
}

/// What a lease still being renewed is held for; None when it is stale or gone.
fn busy(path: &Path) -> Option<String> {
    let age = fs::metadata(path).ok()?.modified().ok()?.elapsed();
    if age.unwrap_or_default() > STALE {
        return None;
    }
    let holder = fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Holder>(&bytes).ok());
    let Some(Holder { pid, purpose }) = holder else {
        return Some("another Mangas process is using the models folder; try again".into());
    };
    let what = match purpose {
        Purpose::Download => "detector support is downloading",
        Purpose::Move => "the data folder is moving",
        Purpose::Remove => "detector support is being removed",
    };
    Some(format!(
        "{what} (Mangas process {pid}); wait for it to finish or cancel it, then try again"
    ))
}

fn start(path: PathBuf, record: Vec<u8>) -> Lease {
    let stop = Arc::new(AtomicBool::new(false));
    let (file, stopped) = (path.clone(), stop.clone());
    let beat = std::thread::spawn(move || loop {
        std::thread::park_timeout(BEAT);
        if stopped.load(Ordering::SeqCst) {
            break;
        }
        // Never recreates the file: a lease deleted with the folder stays gone.
        let renewed = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&file)
            .and_then(|mut f| f.write_all(&record));
        if let Err(e) = renewed.filter_not_found() {
            log::warn!("renew {}: {e}", file.display());
        }
    });
    Lease {
        path,
        stop,
        beat: Some(beat),
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(beat) = self.beat.take() {
            beat.thread().unpark();
            if beat.join().is_err() {
                log::warn!("lease renewal thread panicked");
            }
        }
        if let Err(e) = remove_if_present(&self.path) {
            log::warn!("release {}: {e}", self.path.display());
        }
    }
}

fn remove_if_present(path: &Path) -> std::io::Result<()> {
    fs::remove_file(path).filter_not_found()
}

trait FilterNotFound {
    fn filter_not_found(self) -> std::io::Result<()>;
}

impl FilterNotFound for std::io::Result<()> {
    fn filter_not_found(self) -> std::io::Result<()> {
        match self {
            Err(e) if e.kind() != ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}
