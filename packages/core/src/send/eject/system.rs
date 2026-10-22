//! The real OS behind [`Backend`]: each strategy on the platform that has it.

use std::path::{Path, PathBuf};

use super::{flush, holders, mount, Backend, StepError, Strategy};

/// The [`Backend`] the app uses.
pub struct SystemBackend;

impl Backend for SystemBackend {
    fn mount_point(&self, path: &Path) -> Option<PathBuf> {
        mount::mount_point(path)
    }

    fn flush(&self, files: &[PathBuf]) -> std::io::Result<()> {
        flush::flush(files)
    }

    fn run(&self, strategy: Strategy, mount: &Path) -> Result<bool, StepError> {
        match strategy {
            Strategy::Udisks => udisks(mount),
            Strategy::Umount => umount(mount),
            Strategy::Diskutil => diskutil(mount),
            Strategy::Windows => windows(mount),
        }
    }

    fn holders(&self, mount: &Path) -> Vec<String> {
        holders::holders(mount)
    }
}

fn unavailable(what: &str) -> Result<bool, StepError> {
    Err(StepError::Unavailable(format!(
        "{what} isn't available here"
    )))
}

#[cfg(target_os = "linux")]
fn udisks(mount: &Path) -> Result<bool, StepError> {
    super::linux::eject(mount)
}

#[cfg(not(target_os = "linux"))]
fn udisks(_: &Path) -> Result<bool, StepError> {
    unavailable("udisks2")
}

#[cfg(target_os = "macos")]
fn diskutil(mount: &Path) -> Result<bool, StepError> {
    super::macos::eject(mount)
}

#[cfg(not(target_os = "macos"))]
fn diskutil(_: &Path) -> Result<bool, StepError> {
    unavailable("diskutil")
}

#[cfg(windows)]
fn windows(mount: &Path) -> Result<bool, StepError> {
    super::windows::eject(mount)
}

#[cfg(not(windows))]
fn windows(_: &Path) -> Result<bool, StepError> {
    unavailable("Windows' device eject")
}

/// `umount <mount>` (the files are already synced). Without root this only works on mounts
/// the user may unmount (`user` in fstab).
#[cfg(unix)]
fn umount(mount: &Path) -> Result<bool, StepError> {
    let out = std::process::Command::new("umount")
        .arg(mount)
        .output()
        .map_err(|e| StepError::Unavailable(format!("umount: {e}")))?;
    if out.status.success() {
        return Ok(false);
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let text: String = stderr
        .trim()
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .take(200)
        .collect();
    Err(if text.contains("busy") {
        StepError::Busy(text)
    } else {
        StepError::Failed(text)
    })
}

#[cfg(not(unix))]
fn umount(_: &Path) -> Result<bool, StepError> {
    unavailable("umount")
}
