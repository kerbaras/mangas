//! Copying a book to a Kindle connected over MTP only (Paperwhite 11th gen and later), where
//! macOS and Windows have no folder to copy into. The copy protocol (`transfer`) runs against
//! the [`MtpStore`] trait: the real device (`mtp-rs`, feature `mtp`, macOS and Windows) or a
//! folder that plays one (QA: [`FAKE_STORAGE`]). Linux keeps the gvfs-mounted folder.

mod folder;
pub mod holder;
#[cfg(all(feature = "mtp", any(target_os = "macos", target_os = "windows")))]
mod real;
mod store;
mod transfer;

use std::path::{Path, PathBuf};
use std::time::Duration;

pub use folder::FolderStore;
pub use store::{Entry, Fault, Handle, MtpStore, OnChunk, UploadFailed};
pub use transfer::{send, Stopped, DOCUMENTS};

use crate::domain::{Error, Result};
use crate::send::usb::{Copied, UsbKindle};

/// QA hook: a folder that plays the Kindle's storage 0, with `MANGAS_FAKE_USB_KINDLE` naming
/// the Kindle on the bus; [`FAKE_PAUSE`] slows each 256 KiB chunk so a cancel can be watched.
pub const FAKE_STORAGE: &str = "MANGAS_FAKE_MTP_STORAGE";
pub const FAKE_PAUSE: &str = "MANGAS_FAKE_MTP_PAUSE_MS";

/// Whether this build copies to MTP: macOS and Windows with feature `mtp`, or the QA folder.
pub fn copyable() -> bool {
    REAL || fake_storage().is_some()
}

const REAL: bool = cfg!(all(
    feature = "mtp",
    any(target_os = "macos", target_os = "windows")
));

fn fake_storage() -> Option<PathBuf> {
    std::env::var_os(FAKE_STORAGE)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Copies `file` into `documents/` on the MTP Kindle `device`, with the same promise as a
/// folder copy (`copy_to_kindle_until`): a failed or cancelled copy leaves no partial book and
/// keeps one already there. A book of the same name and size is `already_there`.
pub fn copy_to_mtp_kindle(
    file: &Path,
    device: &UsbKindle,
    cancelled: &(dyn Fn() -> bool + Sync),
    on_progress: &mut (dyn FnMut(u64, u64) + Send),
) -> Result<Copied> {
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| Error::Send(format!("{} is not a file", file.display())))?;
    let size = std::fs::metadata(file)
        .map_err(|e| Error::Send(format!("Can't read {}: {e}", file.display())))?
        .len();
    futures::executor::block_on(async {
        let store: Box<dyn MtpStore> = match fake_storage() {
            Some(root) => Box::new(FolderStore::new(&device.name, &root, fake_pause())),
            None => open(&device.name).await?,
        };
        send(&*store, file, &name, size, cancelled, on_progress)
            .await
            .map_err(|Stopped(fault)| stopped(&device.name, fault))
    })
}

fn fake_pause() -> Duration {
    let ms = std::env::var(FAKE_PAUSE).ok().and_then(|v| v.parse().ok());
    Duration::from_millis(ms.unwrap_or(0))
}

#[cfg(all(feature = "mtp", any(target_os = "macos", target_os = "windows")))]
async fn open(device: &str) -> Result<Box<dyn MtpStore>> {
    match real::open(device).await {
        Ok(store) => Ok(Box::new(store)),
        Err(real::OpenFailed::Held) => Err(held(device, holder::exclusive_owner().as_deref())),
        Err(real::OpenFailed::Other(why)) => Err(Error::Send(format!(
            "Couldn't open {device} over MTP ({why}); unplug it, plug it back in and unlock it"
        ))),
    }
}

#[cfg(not(all(feature = "mtp", any(target_os = "macos", target_os = "windows"))))]
async fn open(device: &str) -> Result<Box<dyn MtpStore>> {
    Err(Error::KindleOverMtp {
        device: device.to_string(),
    })
}

/// [`Error::KindleHeld`] for `device`, held by `owner` (`UsbExclusiveOwner`, if known).
pub fn held(device: &str, owner: Option<&str>) -> Error {
    let holder::Holder { who, fix } = holder::holder(owner);
    Error::KindleHeld {
        device: device.to_string(),
        holder: who,
        fix,
    }
}

/// One sentence per way a copy stops, in the folder copy's words.
pub fn stopped(device: &str, fault: Fault) -> Error {
    match fault {
        Fault::Cancelled => Error::Cancelled,
        Fault::Full => Error::DeviceFull {
            device: device.to_string(),
        },
        Fault::Disconnected => Error::Send(format!(
            "{device} is no longer connected; plug it in and unlock it"
        )),
        Fault::Denied => Error::Send(format!(
            "{device} refused the copy; unlock it and check it isn't busy, then try again"
        )),
        Fault::Unsupported => Error::Send(format!("{device} doesn't support this copy over MTP")),
        Fault::Other(why) => Error::Send(format!("Couldn't copy to {device}: {why}")),
    }
}
