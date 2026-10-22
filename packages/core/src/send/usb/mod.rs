//! Send to Kindle over USB: find a Kindle mounted as a drive (or over MTP through gvfs on
//! Linux) and copy the book into its `documents/` folder. A Kindle on the bus with no mounted
//! folder (MTP only on macOS/Windows) is told apart from no Kindle at all.

mod bus;
mod copy;
mod detect;
mod formats;
mod scan;

use std::path::{Path, PathBuf};

pub use bus::{usb_kindles, UsbKindle, AMAZON_VENDOR_ID};
pub use copy::{copy_to_kindle, Copied};
pub use detect::{
    find_kindles, find_kindles_in, kindle_at, looks_like_kindle, mount_roots, KindleVolume,
};
pub use formats::{admit, sideload, usb_verdict, Sideload, UsbVerdict, SIDELOAD};
pub use scan::{decide, scan_kindles, KindleScan, Reach};

use crate::domain::{Error, Result};

/// Follows the device name for a Kindle connected over MTP only.
pub const MTP_HINT: &str = "connects over MTP, which mangas can't copy to on this computer yet";

/// The Kindle mounted at `root` when given (any folder with `documents/`), otherwise the first
/// one `found`. With none mounted, [`Error::KindleOverMtp`] when a Kindle is on the bus.
pub fn choose(root: Option<&Path>, found: KindleScan) -> Result<KindleVolume> {
    if let Some(root) = root {
        return kindle_at(root).ok_or_else(|| {
            Error::Send(format!(
                "Device not found: {} has no documents folder",
                root.display()
            ))
        });
    }
    match found.reach() {
        Reach::Mounted(volume) => Ok(volume),
        Reach::MtpOnly(device) => Err(Error::KindleOverMtp {
            device: device.name,
        }),
        Reach::Nothing => Err(Error::Send(
            "Device not found: connect your Kindle over USB and unlock it".to_string(),
        )),
    }
}

/// The Kindle a [`choose`] result names, for the format table: the mounted one, or the one on
/// the bus over MTP only (no copy, but its format verdict still stands). None with no Kindle.
pub fn chosen_name(chosen: &Result<KindleVolume>) -> Option<&str> {
    match chosen {
        Ok(volume) => Some(&volume.name),
        Err(Error::KindleOverMtp { device }) => Some(device),
        Err(_) => None,
    }
}

/// Where the books named like `files` are in `volume`'s documents folder.
pub fn on_kindle(volume: &KindleVolume, files: &[PathBuf]) -> Vec<PathBuf> {
    let names = files.iter().filter_map(|f| f.file_name());
    names.map(|name| volume.documents.join(name)).collect()
}

/// The Kindle a copy of `files` went to: the one at `root` when given, else the connected one
/// holding all of them (another Kindle may be listed first), else as [`choose`].
pub fn choose_holding(
    root: Option<&Path>,
    found: KindleScan,
    files: &[PathBuf],
) -> Result<KindleVolume> {
    let holds = |v: &KindleVolume| {
        let books = on_kindle(v, files);
        !books.is_empty() && books.iter().all(|b| b.is_file())
    };
    match (root, found.volumes.iter().find(|v| holds(v))) {
        (None, Some(volume)) => Ok(volume.clone()),
        _ => choose(root, found),
    }
}
