//! A child process gets a copy of every file open at its fork, and on Unix a copy of a locked
//! lease file keeps the lock until the child execs: a lease released in that window would still
//! read as busy, with no owner. So lease files open and close under a lock that starting a child
//! takes exclusively, and the child closes its copies before it execs.

use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::process::{Child, Command};
use std::sync::{PoisonError, RwLock};

static SPAWNING: RwLock<()> = RwLock::new(());

/// An open lease file, closed in every child this process starts (see [`spawn`]).
pub(super) struct LeaseFile(ManuallyDrop<File>);

impl LeaseFile {
    pub(super) fn open(open: impl FnOnce() -> io::Result<File>) -> io::Result<LeaseFile> {
        let _no_spawn = SPAWNING.read().unwrap_or_else(PoisonError::into_inner);
        let file = open()?;
        #[cfg(unix)]
        fds::add(&file);
        Ok(LeaseFile(ManuallyDrop::new(file)))
    }

    pub(super) fn file(&self) -> &File {
        &self.0
    }
}

impl Drop for LeaseFile {
    fn drop(&mut self) {
        // Closed under the guard: a child started between forgetting the fd and closing it
        // would keep the lock.
        let _no_spawn = SPAWNING.read().unwrap_or_else(PoisonError::into_inner);
        #[cfg(unix)]
        fds::remove(&self.0);
        // SAFETY: `self.0` is never used again; this is the only place it is dropped.
        unsafe { ManuallyDrop::drop(&mut self.0) }
    }
}

/// Starts `command`; no child it starts holds a lease of this process, even for a moment.
pub fn spawn(command: &mut Command) -> io::Result<Child> {
    let _spawning = SPAWNING.write().unwrap_or_else(PoisonError::into_inner);
    #[cfg(unix)]
    fds::close_in_child(command);
    // Returns once the child has exec'd (or failed to), so with every copy closed.
    command.spawn()
}

#[cfg(unix)]
mod fds {
    use std::fs::File;
    use std::os::fd::{AsRawFd, RawFd};
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use std::sync::{Mutex, PoisonError};

    static OPEN: Mutex<Vec<RawFd>> = Mutex::new(Vec::new());

    pub(super) fn add(file: &File) {
        OPEN.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(file.as_raw_fd());
    }

    pub(super) fn remove(file: &File) {
        let mut open = OPEN.lock().unwrap_or_else(PoisonError::into_inner);
        open.retain(|&fd| fd != file.as_raw_fd());
    }

    /// The child's stdin, stdout and stderr must all be set (null or piped): they are put in
    /// place before this runs, over any lease that had one of their numbers here.
    pub(super) fn close_in_child(command: &mut Command) {
        let mut fds = OPEN.lock().unwrap_or_else(PoisonError::into_inner).clone();
        fds.retain(|&fd| fd > 2);
        // SAFETY: between fork and exec the closure only calls `close` (async-signal-safe) on
        // a list built before the fork, and every lease file is close-on-exec anyway.
        unsafe {
            command.pre_exec(move || {
                for &fd in &fds {
                    libc::close(fd);
                }
                Ok(())
            });
        }
    }
}
