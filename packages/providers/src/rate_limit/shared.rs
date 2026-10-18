//! The cross-process half of a [`super::TokenBucket`]: its state in a small file, read and
//! written under an exclusive file lock, so the CLI and the desktop app (or two CLI runs) draw
//! from one budget per host. The format is `tokens last_unix_millis`; a missing or unreadable
//! file is a full bucket.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::bucket::refill;

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

    /// Locks the file, hands `(tokens, elapsed since last write)` to `step`, stores its tokens
    /// stamped now. The lock is released when the file closes.
    fn update(
        &self,
        capacity: f64,
        step: impl FnOnce((f64, Duration)) -> (f64, Duration),
    ) -> io::Result<Duration> {
        let mut file = open(&self.path)?;
        file.lock()?;
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
