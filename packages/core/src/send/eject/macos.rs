//! `diskutil eject <mount>`: unmounts every volume of the disk and ejects it, like Finder.

use std::path::Path;
use std::process::Command;

use super::{dissenter, StepError};

pub(super) fn eject(mount: &Path) -> Result<bool, StepError> {
    let out = Command::new("diskutil")
        .arg("eject")
        .arg(mount)
        .output()
        .map_err(|e| StepError::Unavailable(format!("diskutil: {e}")))?;
    if out.status.success() {
        return Ok(true);
    }
    let text = format!(
        "{} {}",
        String::from_utf8_lossy(&out.stdout).trim(),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    let text = text.trim().to_string();
    let busy = dissenter(&text).is_some() || text.contains("in use") || text.contains("busy");
    Err(if busy {
        StepError::Busy(text)
    } else {
        StepError::Failed(text)
    })
}
