//! What a USB delivery finds: a mounted Kindle folder, a Kindle on the bus with no folder (MTP
//! only, so no copy is possible), or nothing.

use serde::Serialize;

use super::bus::{usb_kindles, UsbKindle};
use super::detect::{find_kindles, KindleVolume};

/// The Kindles a USB delivery can see. `mtp` is only filled when no folder is mounted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct KindleScan {
    pub volumes: Vec<KindleVolume>,
    pub mtp: Vec<UsbKindle>,
}

/// What a scan means for a copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reach {
    /// No Kindle at all: "Device not found".
    Nothing,
    /// A mounted folder to copy into (a mass-storage drive, or MTP through gvfs on Linux).
    Mounted(KindleVolume),
    /// On the bus but with no folder: connected over MTP, which this app can't copy to.
    MtpOnly(UsbKindle),
}

/// Mounted folders first; the bus is only listed when none is mounted.
pub fn scan_kindles() -> KindleScan {
    let volumes = find_kindles();
    let mtp = if volumes.is_empty() {
        usb_kindles()
    } else {
        Vec::new()
    };
    decide(volumes, mtp)
}

/// Pure: a mounted folder wins (a mass-storage Kindle is on the bus too); a bus device only
/// counts when nothing is mounted.
pub fn decide(volumes: Vec<KindleVolume>, on_bus: Vec<UsbKindle>) -> KindleScan {
    let mtp = if volumes.is_empty() {
        on_bus
    } else {
        Vec::new()
    };
    KindleScan { volumes, mtp }
}

impl KindleScan {
    /// Where a copy can go, or why it can't.
    pub fn reach(self) -> Reach {
        match (self.volumes.into_iter().next(), self.mtp.into_iter().next()) {
            (Some(volume), _) => Reach::Mounted(volume),
            (None, Some(device)) => Reach::MtpOnly(device),
            (None, None) => Reach::Nothing,
        }
    }
}
