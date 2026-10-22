//! Safe removal of a mounted Kindle: flush what was copied, then unmount and eject the drive the
//! way the OS's own "Eject" does. Which steps run and what the user reads is decided here and in
//! [`plan`] (pure); the OS calls live behind [`Backend`] ([`SystemBackend`] in the app).

mod flush;
mod holders;
mod mount;
mod plan;
mod system;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
mod windows_pnp;
#[cfg(windows)]
mod windows_vhd;

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::domain::{Error, Result};
use crate::send::KindleVolume;

pub(crate) use flush::sync_dir;
pub use holders::{parse_fuser, parse_lsof};
pub use mount::containing_mount;
pub use plan::{
    backing_path, busy_text, dbus_error, dissenter, ejectable, own_volume, pnp_veto, strategies,
    Os, StepError, Strategy, MTP_NO_EJECT, STILL_THERE,
};
pub use system::SystemBackend;

/// QA hook: `udisks` or `umount` forces that strategy on Linux (the udisks2-absent fallback
/// can be run on a machine that has udisks2).
pub const STRATEGY_ENV: &str = "MANGAS_EJECT_STRATEGY";

/// A Kindle that can be unplugged now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ejected {
    pub device: String,
    pub strategy: Strategy,
    /// The drive was also powered off or its media ejected; `false` when only its volume was
    /// unmounted (still safe to unplug).
    pub powered_off: bool,
}

/// The OS side of an eject, faked in tests.
pub trait Backend {
    /// The mount point holding `path`.
    fn mount_point(&self, path: &Path) -> Option<PathBuf>;
    /// Writes `files` (and their folders where the OS allows) to the device, then the whole
    /// cache (`sync`) on Unix.
    fn flush(&self, files: &[PathBuf]) -> std::io::Result<()>;
    /// Unmounts and ejects the volume at `mount`; `Ok(true)` when the drive was powered off too.
    fn run(&self, strategy: Strategy, mount: &Path) -> std::result::Result<bool, StepError>;
    /// Names of the programs with a file open under `mount`.
    fn holders(&self, mount: &Path) -> Vec<String>;
}

/// Flushes `copied` and ejects `kindle` with the OS's own tools.
pub fn eject(kindle: &KindleVolume, copied: &[PathBuf]) -> Result<Ejected> {
    let forced = std::env::var(STRATEGY_ENV)
        .ok()
        .and_then(|s| Strategy::parse(&s));
    eject_with(&SystemBackend, kindle, copied, forced)
}

/// [`eject`] over any [`Backend`], trying each of the OS's [`strategies`] in turn.
pub fn eject_with(
    backend: &dyn Backend,
    kindle: &KindleVolume,
    copied: &[PathBuf],
    forced: Option<Strategy>,
) -> Result<Ejected> {
    if !kindle.ejectable {
        return Err(Error::Send(format!("{} {MTP_NO_EJECT}", kindle.name)));
    }
    let device = format!("{} at {}", kindle.name, kindle.root.display());
    let refused = |reason: String| Error::EjectRefused {
        device: device.clone(),
        reason: reason.trim_end_matches('.').to_string(),
    };
    let mount = backend
        .mount_point(&kindle.root)
        .ok_or_else(|| refused("it's no longer mounted".into()))?;
    if !own_volume(&mount, &kindle.root) {
        let why = format!(
            "it isn't a drive of its own (it's inside {})",
            mount.display()
        );
        return Err(refused(why));
    }
    backend
        .flush(copied)
        .map_err(|e| refused(format!("the copy couldn't be written to it ({e})")))?;
    let mut why = String::from("this system has no way to eject drives");
    for strategy in strategies(Os::current(), forced) {
        match backend.run(strategy, &mount) {
            Ok(powered_off) => {
                return Ok(Ejected {
                    device: kindle.name.clone(),
                    strategy,
                    powered_off,
                })
            }
            Err(StepError::Unavailable(reason)) => {
                log::info!(
                    "eject {}: {strategy:?} unavailable: {reason}",
                    mount.display()
                );
                why = reason;
            }
            Err(StepError::Busy(detail)) => {
                return Err(Error::VolumeBusy {
                    holders: backend.holders(&mount),
                    device,
                    detail,
                })
            }
            Err(StepError::Failed(reason)) => return Err(refused(reason)),
        }
    }
    Err(refused(why))
}
