//! Where a USB copy goes: a mounted folder, or straight to a Kindle over MTP where this build
//! can (macOS and Windows with feature `mtp`; Linux reaches MTP Kindles through gvfs folders).

use std::path::Path;

use super::{choose, copy_to_kindle_until, Copied, KindleScan, KindleVolume, UsbKindle};
use crate::domain::Result;
use crate::send::mtp;

/// A Kindle a copy can reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Volume(KindleVolume),
    Mtp(UsbKindle),
}

impl Target {
    pub fn name(&self) -> &str {
        match self {
            Target::Volume(v) => &v.name,
            Target::Mtp(d) => &d.name,
        }
    }
}

/// [`choose`], except that a Kindle on MTP only is a target when [`mtp::copyable`].
pub fn target(root: Option<&Path>, found: KindleScan) -> Result<Target> {
    pick(root, found, mtp::copyable())
}

/// Pure: [`target`] with `copyable` given.
pub fn pick(root: Option<&Path>, found: KindleScan, copyable: bool) -> Result<Target> {
    match (root, found.volumes.is_empty(), found.mtp.first()) {
        (None, true, Some(device)) if copyable => Ok(Target::Mtp(device.clone())),
        _ => choose(root, found).map(Target::Volume),
    }
}

/// The Kindle a [`target`] result names, for the format table (see [`super::chosen_name`]).
pub fn target_name(chosen: &Result<Target>) -> Option<&str> {
    match chosen {
        Ok(target) => Some(target.name()),
        Err(crate::Error::KindleOverMtp { device }) => Some(device),
        Err(_) => None,
    }
}

/// Copies `file` to `target`; stops at the next chunk once `cancelled()` ([`crate::Error::Cancelled`]).
pub fn deliver(
    file: &Path,
    target: &Target,
    cancelled: &(dyn Fn() -> bool + Sync),
    on_progress: &mut (dyn FnMut(u64, u64) + Send),
) -> Result<Copied> {
    match target {
        Target::Volume(kindle) => copy_to_kindle_until(file, kindle, cancelled, on_progress),
        Target::Mtp(device) => mtp::copy_to_mtp_kindle(file, device, cancelled, on_progress),
    }
}
