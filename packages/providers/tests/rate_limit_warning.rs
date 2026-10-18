//! A bucket file that cannot be written (read-only data dir) is a warning once per host per
//! process, not once per request; later failures only go to the debug log.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;

use log::{Level, LevelFilter, Log, Metadata, Record};
use mangas_providers::HostLimiters;

struct Capture(Mutex<Vec<(Level, String)>>);

impl Log for Capture {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        let line = (record.level(), record.args().to_string());
        self.0.lock().unwrap().push(line);
    }
    fn flush(&self) {}
}

static LOGS: Capture = Capture(Mutex::new(Vec::new()));

fn fallbacks(level: Level) -> Vec<String> {
    let logs = LOGS.0.lock().unwrap();
    logs.iter()
        .filter(|(l, m)| *l == level && m.contains("pacing in this process only"))
        .map(|(_, m)| m.clone())
        .collect()
}

#[tokio::test]
async fn a_read_only_data_dir_warns_once_per_host() {
    log::set_logger(&LOGS).unwrap();
    log::set_max_level(LevelFilter::Debug);
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
    if std::fs::write(dir.path().join("probe"), b"").is_ok() {
        eprintln!("skipped: running as a user who can write a read-only dir (root)");
        return;
    }
    let limiters = HostLimiters::default();
    limiters.share_across_processes(dir.path().to_path_buf());
    let one = limiters.bucket("https://api.one.test/search", 100);
    let two = limiters.bucket("https://api.two.test/manga", 100);
    for _ in 0..5 {
        one.acquire().await;
        limiters
            .bucket("https://api.one.test/chapter", 100)
            .acquire()
            .await;
    }
    two.acquire().await;
    two.acquire().await;

    let warned = fallbacks(Level::Warn);
    assert_eq!(warned.len(), 2, "one warning per host: {warned:?}");
    assert!(warned[0].contains("api.one.test"), "{warned:?}");
    assert!(warned[1].contains("api.two.test"), "{warned:?}");
    assert_eq!(
        fallbacks(Level::Debug).len(),
        9 + 1,
        "the rest at debug level"
    );
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
}
