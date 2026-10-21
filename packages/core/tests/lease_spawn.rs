//! A child process started while this process releases a lease must not keep it locked.

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mangas_core::ml::lease::{acquire, spawn_child, Purpose};

#[test]
fn a_released_lease_is_free_while_children_start() {
    let dir = tempfile::tempdir().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let spawner = {
        let stop = stop.clone();
        std::thread::spawn(move || {
            let mut children = 0;
            while !stop.load(Ordering::Relaxed) {
                let mut command = Command::new("sleep");
                command.arg("1").stdin(Stdio::null()).stdout(Stdio::null());
                let mut child = spawn_child(&mut command).unwrap();
                children += 1;
                // Reaped in the background: the child lives on after exec, lease-free.
                std::thread::spawn(move || child.wait());
            }
            children
        })
    };
    let started = Instant::now();
    let mut rounds = 0;
    while started.elapsed() < Duration::from_secs(3) {
        drop(acquire(dir.path(), Purpose::Download).unwrap());
        let again = acquire(dir.path(), Purpose::Download);
        assert!(again.is_ok(), "round {rounds}: {:?}", again.err());
        rounds += 1;
    }
    stop.store(true, Ordering::Relaxed);
    let children = spawner.join().unwrap();
    assert!(
        children > 50 && rounds > 50,
        "{children} children, {rounds} rounds"
    );
}
