//! A live process that holds the shared bucket lock (hung mid-update) stalls the others for at
//! most two seconds: they warn once, naming the file, and pace in memory. A killed holder still
//! leaves no stuck lock. The holder is this test binary re-run as `child_holds_lock`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use mangas_providers::TokenBucket;

const CHILD_ENV: &str = "MANGAS_RATE_LIMIT_HOLDER";

static LOGS: Mutex<String> = Mutex::new(String::new());

struct Sink;

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        LOGS.lock().unwrap().push_str(&String::from_utf8_lossy(buf));
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn capture_logs() {
    let subscriber = tracing_subscriber::fmt()
        .with_writer(|| Sink)
        .with_ansi(false)
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
}

fn lines(level: &str) -> Vec<String> {
    let logs = LOGS.lock().unwrap();
    logs.lines()
        .filter(|l| l.contains(level) && l.contains("rate limit lock"))
        .map(str::to_string)
        .collect()
}

fn held_marker(path: &Path) -> PathBuf {
    path.with_extension("held")
}

/// Locks the bucket file, says so with a marker file, then hangs while holding it.
#[test]
fn child_holds_lock() {
    let Ok(path) = std::env::var(CHILD_ENV) else {
        return;
    };
    let path = PathBuf::from(path);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .unwrap();
    file.lock().unwrap();
    std::fs::write(held_marker(&path), b"").unwrap();
    std::thread::sleep(Duration::from_secs(60));
}

fn holder(path: &Path) -> Child {
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_holds_lock", "--test-threads=1"])
        .env(CHILD_ENV, path)
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while !held_marker(path).exists() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "holder never locked"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    child
}

#[tokio::test]
async fn a_live_holder_costs_two_seconds_then_a_killed_one_nothing() {
    capture_logs();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("host_443.bucket");
    let mut child = holder(&path);

    let bucket = TokenBucket::new(10, 10);
    bucket.share_in(path.clone());
    let start = Instant::now();
    bucket.acquire().await;
    let first = start.elapsed();
    assert!(first >= Duration::from_millis(1950), "{first:?}");
    assert!(first < Duration::from_secs(4), "{first:?}");
    let start = Instant::now();
    for _ in 0..4 {
        bucket.acquire().await;
        bucket.pause(Duration::ZERO).await;
    }
    let rest = start.elapsed();
    assert!(
        rest < Duration::from_millis(500),
        "paced in memory: {rest:?}"
    );

    let warned = lines("WARN");
    assert_eq!(warned.len(), 1, "one warning: {warned:?}");
    assert!(
        warned[0].contains(&path.display().to_string()),
        "{warned:?}"
    );
    assert!(
        warned[0].contains("pacing in this process only"),
        "{warned:?}"
    );
    assert!(
        std::fs::read_to_string(&path).unwrap().is_empty(),
        "file untouched"
    );

    child.kill().unwrap();
    child.wait().unwrap();
    let fresh = TokenBucket::new(10, 10);
    fresh.share_in(path.clone());
    let start = Instant::now();
    fresh.acquire().await;
    assert!(
        start.elapsed() < Duration::from_millis(500),
        "no stuck lock"
    );
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.starts_with("9 "),
        "token taken from the file: {text:?}"
    );
}
