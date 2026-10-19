//! The detector status checked in a short-lived child process (the shipped binary is also the CLI:
//! `<exe> ml status --json`), so this process never maps the ONNX Runtime just to show a status.
//! Windows keeps a loaded DLL locked until the process exits, which made Remove need a restart.

use std::ffi::OsString;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use super::{checked_status, runtime, status, RuntimeStatus, Status};

/// Long enough for a cold load from a slow disk; a stuck child is killed and reported.
const TIMEOUT: Duration = Duration::from_secs(60);

/// The arguments that make the CLI print [`checked_status`] of `models_dir` as JSON.
pub fn args(models_dir: &Path) -> Vec<OsString> {
    let fixed = ["ml", "status", "--json", "--models-dir"];
    let mut args: Vec<OsString> = fixed.iter().map(OsString::from).collect();
    args.push(models_dir.into());
    args
}

/// [`checked_status`] with the load tried in `exe` (this program run as the CLI). Once this
/// process has loaded the runtime (or failed to) for a detector run, its own answer is the one
/// that counts, and asking it maps nothing new. If `exe` can't be started, it checks in-process.
pub fn probed_status(models_dir: &Path, exe: &Path) -> Status {
    let now = status(models_dir);
    if runtime::attempted() {
        return checked_status(models_dir);
    }
    if !now.downloaded {
        return now;
    }
    match spawn(models_dir, exe) {
        Ok(child) => from_child(now, wait(child, TIMEOUT)),
        Err(e) => {
            tracing::warn!("detector status child {}: {e}", exe.display());
            checked_status(models_dir)
        }
    }
}

fn spawn(models_dir: &Path, exe: &Path) -> std::io::Result<Child> {
    let mut command = Command::new(exe);
    command
        .args(args(models_dir))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    no_window(&mut command);
    command.spawn()
}

#[cfg(windows)]
fn no_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn no_window(_: &mut Command) {}

/// Waits up to `timeout`, then kills the child. Its output is a few lines, well under a pipe's
/// buffer, so reading after the exit can't deadlock.
fn wait(mut child: Child, timeout: Duration) -> std::io::Result<Output> {
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > timeout {
            // Already exited or unkillable: the error below says what happened either way.
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("no answer in {} s", timeout.as_secs()),
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        out.read_to_end(&mut stdout)?;
    }
    if let Some(mut err) = child.stderr.take() {
        err.read_to_end(&mut stderr)?;
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// The child's status, or `now` with the runtime failed and the child's own words: a library
/// that crashes the loader is as broken as one that refuses to load.
pub fn from_child(now: Status, output: std::io::Result<Output>) -> Status {
    let reason = match output {
        Ok(out) if out.status.success() => match serde_json::from_slice::<Status>(&out.stdout) {
            Ok(child) => return child,
            Err(e) => format!("detector status check: unreadable answer ({e})"),
        },
        Ok(out) => {
            let said = String::from_utf8_lossy(&out.stderr);
            let said = said.trim().trim_start_matches("error: ");
            format!("detector status check failed ({}): {said}", out.status)
        }
        Err(e) => format!("detector status check: {e}"),
    };
    failed(now, reason)
}

fn failed(now: Status, reason: String) -> Status {
    let version = match &now.runtime {
        RuntimeStatus::Downloaded { version }
        | RuntimeStatus::NotDownloaded { version }
        | RuntimeStatus::Failed { version, .. }
        | RuntimeStatus::RemovePending { version, .. } => version.clone(),
        RuntimeStatus::Unavailable => return now,
    };
    Status {
        downloaded: false,
        runtime: RuntimeStatus::Failed { version, reason },
        ..now
    }
}

#[cfg(test)]
mod tests;
