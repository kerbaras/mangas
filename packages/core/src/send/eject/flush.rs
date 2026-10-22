//! Every copied byte on the device before it's ejected: each file, its folder, then `sync`.

use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// Writes `dir`'s entries (a rename into it) to the device. Unix only: Windows can't open a
/// folder for writing, and its dismount flushes the volume anyway.
pub(crate) fn sync_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    match File::open(dir).and_then(|d| d.sync_all()) {
        // Some filesystems can't fsync a folder; the file itself is already synced.
        Err(e) if matches!(e.kind(), ErrorKind::InvalidInput | ErrorKind::Unsupported) => {}
        other => other?,
    }
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

/// fsyncs each of `files` (one that's gone is skipped) and its folder, then `sync` on Unix.
pub(super) fn flush(files: &[PathBuf]) -> std::io::Result<()> {
    for file in files {
        // Windows' FlushFileBuffers needs a handle open for writing.
        match File::options().read(true).write(cfg!(windows)).open(file) {
            Ok(f) => f.sync_all()?,
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        }
        if let Some(dir) = file.parent() {
            sync_dir(dir)?;
        }
    }
    #[cfg(unix)]
    // SAFETY: sync(2) takes no arguments and can't fail.
    unsafe {
        libc::sync()
    };
    Ok(())
}
