//! The cross-process half of a [`super::TokenBucket`]: its state in a small file, read and
//! written under an exclusive file lock, so the CLI and the desktop app (or two CLI runs) draw
//! from one budget per host. The format is `tokens last_unix_millis`; a missing or unreadable
//! file is a full bucket. A process that hangs while holding the lock must not stall the others:
//! the lock is waited on for at most [`LOCK_WAIT`], then the call fails with
//! [`io::ErrorKind::TimedOut`] and the caller paces in memory.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::bucket::refill;

/// Longest wait for another process to release the bucket file.
pub const LOCK_WAIT: Duration = Duration::from_secs(2);
/// How often a held lock is retried within [`LOCK_WAIT`].
const LOCK_POLL: Duration = Duration::from_millis(10);

#[derive(Debug, Clone)]
pub struct SharedState {
    path: PathBuf,
}

impl SharedState {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Takes a token if one is available (returns zero) or says how long until the next one.
    pub fn take(&self, capacity: f64, per_second: f64) -> io::Result<Duration> {
        self.update(capacity, |tokens| {
            let (tokens, wait) = refill(tokens.0, tokens.1, capacity, per_second);
            match wait.is_zero() {
                true => (tokens - 1.0, wait),
                false => (tokens, wait),
            }
        })
    }

    /// Puts the bucket in debt so the next token comes after `wait`, for every process.
    pub fn pause(&self, wait: Duration, capacity: f64, per_second: f64) -> io::Result<()> {
        self.update(capacity, |tokens| {
            let (tokens, _) = refill(tokens.0, tokens.1, capacity, per_second);
            (tokens.min(-wait.as_secs_f64() * per_second), Duration::ZERO)
        })
        .map(|_| ())
    }

    /// Locks the file (waiting at most [`LOCK_WAIT`]), hands `(tokens, elapsed since last
    /// write)` to `step`, stores its tokens stamped now. The lock is released when the file closes.
    fn update(
        &self,
        capacity: f64,
        step: impl FnOnce((f64, Duration)) -> (f64, Duration),
    ) -> io::Result<Duration> {
        let mut file = open(&self.path)?;
        lock_within(&file, LOCK_WAIT)?;
        let mut text = String::new();
        file.read_to_string(&mut text)?;
        let now = now_millis();
        let (tokens, last) = parse(&text).unwrap_or((capacity, now));
        let elapsed = Duration::from_millis(now.saturating_sub(last));
        let (tokens, wait) = step((tokens, elapsed));
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        file.write_all(format!("{tokens} {now}").as_bytes())?;
        Ok(wait)
    }
}

/// Takes the exclusive lock, retrying while another holder has it, for at most `wait`.
fn lock_within(file: &File, wait: Duration) -> io::Result<()> {
    let deadline = Instant::now() + wait;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(TryLockError::Error(e)) => return Err(e),
            Err(TryLockError::WouldBlock) if Instant::now() >= deadline => {
                let held = format!("held by another process for over {}s", wait.as_secs());
                return Err(io::Error::new(io::ErrorKind::TimedOut, held));
            }
            Err(TryLockError::WouldBlock) => std::thread::sleep(LOCK_POLL),
        }
    }
}

fn open(path: &Path) -> io::Result<File> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

/// `tokens last_unix_millis`, or `None` for an empty or damaged file.
pub fn parse(text: &str) -> Option<(f64, u64)> {
    let mut parts = text.split_whitespace();
    let tokens = parts
        .next()?
        .parse::<f64>()
        .ok()
        .filter(|t| t.is_finite())?;
    let last = parts.next()?.parse().ok()?;
    Some((tokens, last))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// A file name for a host key (`host:port`): safe on every OS.
pub fn file_name(host_key: &str) -> String {
    let safe: String = host_key
        .chars()
        .map(
            |c| match c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                true => c,
                false => '_',
            },
        )
        .collect();
    format!("{safe}.bucket")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_what_it_writes_and_rejects_garbage() {
        assert_eq!(parse("1.5 1700000000000"), Some((1.5, 1_700_000_000_000)));
        assert_eq!(parse("-2 5"), Some((-2.0, 5)));
        assert_eq!(parse(""), None);
        assert_eq!(parse("NaN 5"), None);
        assert_eq!(parse("1"), None);
    }

    #[test]
    fn file_names_are_plain() {
        assert_eq!(
            file_name("api.mangadex.org:443"),
            "api.mangadex.org_443.bucket"
        );
        assert_eq!(file_name("[::1]:80"), "___1__80.bucket");
    }
}
