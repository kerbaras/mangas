//! Copying a book onto a mounted Kindle, with byte progress.

use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::KindleVolume;
use crate::domain::{Error, Result};

const CHUNK: usize = 1 << 20;

/// Where the book is on the Kindle; `already_there` when `file` was that book, so nothing was
/// copied.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Copied {
    pub path: PathBuf,
    pub already_there: bool,
}

/// Copies `file` into the Kindle's `documents/` folder, replacing a book of the same name. `on_progress(done, total)` gets the bytes copied so far. The data
/// goes to a hidden `.part` file that is flushed and then renamed over the target, so a failed copy
/// leaves no partial file and keeps a book that was already there.
pub fn copy_to_kindle(
    file: &Path,
    kindle: &KindleVolume,
    on_progress: &mut dyn FnMut(u64, u64),
) -> Result<Copied> {
    let name = file
        .file_name()
        .ok_or_else(|| Error::Send(format!("{} is not a file", file.display())))?;
    if !kindle.documents.is_dir() {
        return Err(Error::Send(format!(
            "{} is no longer connected; plug it in and unlock it",
            kindle.name
        )));
    }
    let dest = kindle.documents.join(name);
    if same_file(file, &dest) {
        let total = std::fs::metadata(file)?.len();
        on_progress(total, total);
        return Ok(Copied {
            path: dest,
            already_there: true,
        });
    }
    let src =
        File::open(file).map_err(|e| Error::Send(format!("Can't read {}: {e}", file.display())))?;
    let part = kindle.documents.join(part_name());
    let dst = File::options()
        .write(true)
        .create_new(true)
        .open(&part)
        .map_err(|e| device_error(kindle, &e))?;
    copy(src, dst, on_progress)
        .and_then(|()| std::fs::rename(&part, &dest))
        .map_err(|e| {
            let _ = std::fs::remove_file(&part);
            device_error(kindle, &e)
        })?;
    Ok(Copied {
        path: dest,
        already_there: false,
    })
}

/// One sentence naming the device and what to do, without the OS text twice.
fn device_error(kindle: &KindleVolume, e: &std::io::Error) -> Error {
    let at = format!("{} at {}", kindle.name, kindle.root.display());
    Error::Send(match e.kind() {
        ErrorKind::ReadOnlyFilesystem | ErrorKind::PermissionDenied => {
            format!("{at} is read-only; eject and reconnect it, or check it isn't locked")
        }
        ErrorKind::StorageFull => format!("{at} is full; delete some books from it and try again"),
        _ => format!("Couldn't copy to {at}: {e}"),
    })
}

/// `.mangas-<pid>-<n>.part`: short, so it fits wherever the book's own name fits, and unique
/// per copy, so two sends of one book never share a file.
fn part_name() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    format!(".mangas-{}-{n}.part", std::process::id())
}

fn same_file(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

fn copy(
    mut src: File,
    mut dst: File,
    on_progress: &mut dyn FnMut(u64, u64),
) -> std::io::Result<()> {
    let total = src.metadata()?.len();
    let mut buf = vec![0u8; CHUNK];
    let mut done = 0u64;
    on_progress(0, total);
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            break;
        }
        dst.write_all(&buf[..n])?;
        done += n as u64;
        on_progress(done, total);
    }
    dst.sync_all()?;
    Ok(())
}
