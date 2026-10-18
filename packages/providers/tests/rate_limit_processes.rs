//! Two processes sharing one bucket file get one budget, not one each. The test re-runs its own
//! binary as the child processes (`child_takes_tokens`, a no-op unless the env var is set).

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use mangas_providers::TokenBucket;

const CHILD_ENV: &str = "MANGAS_RATE_LIMIT_CHILD";
/// Burst and rate of the shared bucket, and the tokens each child takes.
const RATE: u32 = 4;
const TAKES: u32 = 4;

#[tokio::test]
async fn child_takes_tokens() {
    let Ok(path) = std::env::var(CHILD_ENV) else {
        return;
    };
    let bucket = TokenBucket::new(RATE, RATE);
    bucket.share_in(path.into());
    for _ in 0..TAKES {
        bucket.acquire().await;
    }
}

fn child(path: &std::path::Path) -> std::process::Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_takes_tokens", "--test-threads=1"])
        .env(CHILD_ENV, path)
        .stdout(Stdio::null())
        .spawn()
        .unwrap()
}

#[test]
fn two_processes_share_one_budget() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("host_443.bucket");
    let start = Instant::now();
    let mut children = [child(&path), child(&path)];
    for c in &mut children {
        assert!(c.wait().unwrap().success());
    }
    // 8 tokens from a burst of 4 refilled at 4/s: at least 1 s. Separate budgets: no wait.
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(950), "{elapsed:?}");
    assert!(path.exists());
}

#[tokio::test]
async fn an_unusable_file_falls_back_to_memory() {
    let dir = tempfile::tempdir().unwrap();
    let bucket = TokenBucket::new(2, 2);
    // A directory where the file should be: every file operation fails.
    bucket.share_in(dir.path().to_path_buf());
    let start = Instant::now();
    for _ in 0..3 {
        bucket.acquire().await;
    }
    assert!(start.elapsed() >= Duration::from_millis(400));
}
