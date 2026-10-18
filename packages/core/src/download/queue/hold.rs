//! Holding the whole queue still while something rewrites the data folder (storage move).

use std::time::{Duration, Instant};

use super::job::JobState;
use crate::domain::{Error, Result};
use crate::library::{self, Library};

/// How long a data-folder move waits for running downloads to stop.
pub const MOVE_SETTLE: Duration = Duration::from_secs(30);

/// A stopping task is aborted on its worker's next tick (500 ms): settled means no job was
/// running over a window longer than that.
const QUIET: Duration = Duration::from_millis(1200);
const POLL: Duration = Duration::from_millis(100);

/// Sets the queue-wide pause (every process stops claiming), waits up to `wait` until no job
/// is running, runs `f`, and restores the pause flag as it was, whatever `f` returns. Refuses
/// with the count of running downloads when they don't settle in time.
pub fn hold_queue<T>(lib: &Library, wait: Duration, f: impl FnOnce() -> Result<T>) -> Result<T> {
    let was_paused = library::queue_paused(lib)?;
    library::set_queue_paused(lib, true)?;
    let outcome = settle(lib, wait).and_then(|()| f());
    library::set_queue_paused(lib, was_paused)?;
    outcome
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
