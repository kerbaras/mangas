//! Holding the whole queue still while something rewrites the data folder (storage move).

use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use super::job::JobState;
use super::ops::now_ms;
use crate::domain::{Error, Result};
use crate::library::{self, Library};

/// How long a data-folder move waits for running downloads to stop.
pub const MOVE_SETTLE: Duration = Duration::from_secs(30);

/// A stopping task is aborted on its worker's next tick (500 ms): settled means no job was
/// running over a window longer than that.
const QUIET: Duration = Duration::from_millis(1200);
const POLL: Duration = Duration::from_millis(100);
/// The hold is a lease, renewed while held, so a crashed move frees the queue on its own.
const LEASE_MS: i64 = 60_000;
const RENEW: Duration = Duration::from_secs(10);

/// Holds the queue (no process claims a job; the user's pause flag is left alone, so "resume
/// all" can't end the hold), waits up to `wait` until no job is running, runs `f`, and releases
/// the hold whatever `f` returns. Refuses with the count of running downloads when they don't
/// settle in time, and while another move holds the queue.
pub fn hold_queue<T>(lib: &Library, wait: Duration, f: impl FnOnce() -> Result<T>) -> Result<T> {
    let now = now_ms();
    if !library::take_queue_hold(lib, now, now + LEASE_MS)? {
        return Err(Error::Other(
            "another data-folder move is in progress; try again when it's done".into(),
        ));
    }
    let (stop, stopped) = mpsc::channel::<()>();
    let outcome = std::thread::scope(|s| {
        s.spawn(move || renew(lib, &stopped));
        let outcome = settle(lib, wait).and_then(|()| f());
        let _ = stop.send(());
        outcome
    });
    library::set_queue_held_until(lib, 0)?;
    outcome
}

fn renew(lib: &Library, stopped: &Receiver<()>) {
    while let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(RENEW) {
        if let Err(e) = library::set_queue_held_until(lib, now_ms() + LEASE_MS) {
            log::warn!("download queue hold: {e}");
        }
    }
}

fn settle(lib: &Library, wait: Duration) -> Result<()> {
    let start = Instant::now();
    let mut quiet_since: Option<Instant> = None;
    loop {
        let running = running(lib)?;
        let now = Instant::now();
        if running == 0 {
            let since = *quiet_since.get_or_insert(now);
            if now - since >= QUIET {
                return Ok(());
            }
        } else {
            quiet_since = None;
        }
        if now - start >= wait {
            return Err(Error::Other(format!(
                "{running} download(s) still running after {} s; let them finish or cancel them \
                 before moving the data folder",
                wait.as_secs()
            )));
        }
        std::thread::sleep(POLL);
    }
}

fn running(lib: &Library) -> Result<usize> {
    Ok(library::list_jobs(lib)?
        .iter()
        .filter(|j| j.state == JobState::Running)
        .count())
}
