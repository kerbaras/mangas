//! The CLI's settling run while another process still downloads one of its chapters.

mod queue_support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use mangas_core::download::queue::{self, JobState, Queue, Scope};
use mangas_core::library::{self, Library};
use queue_support::{eventually, fast_config, follow, states, FakeRunner};

/// A worker of "another process" (its own owner) downloading the one chapter queued.
async fn running_elsewhere(lib: &Arc<Library>) -> tokio::task::JoinHandle<()> {
    queue::enqueue(lib, "a", follow(lib, "a", 1), false).unwrap();
    let runner = FakeRunner {
        delay: Mutex::new(Duration::from_millis(600)),
        ..Default::default()
    };
    let other = Arc::new(Queue::new(lib.clone(), Arc::new(runner), fast_config()));
    let task = tokio::spawn(async move {
        let _ = other.run(Scope::All, false, &|_| {}).await;
    });
    eventually(|| queue::list(lib).unwrap()[0].state == JobState::Running).await;
    task
}

fn idle_worker(lib: &Arc<Library>) -> Queue {
    Queue::new(lib.clone(), Arc::new(FakeRunner::default()), fast_config())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_move_hold_does_not_pass_a_chapter_running_elsewhere_off_as_settled() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    let other = running_elsewhere(&lib).await;
    let now = queue::now_ms();
    assert!(library::take_queue_hold(&lib, now, now + 60_000).unwrap());
    let release = lib.clone();
    let mover = tokio::spawn(async move {
        eventually(|| queue::list(&release).unwrap()[0].state == JobState::Done).await;
        library::set_queue_held_until(&release, 0).unwrap();
    });
    let worker = idle_worker(&lib);
    let settled = queue::run_until_settled(&worker, Scope::All, &|_| {}, &|| {})
        .await
        .unwrap();
    assert_eq!(states(&settled.jobs), [JobState::Done]);
    assert!(!settled.paused);
    mover.await.unwrap();
    other.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_queue_wide_pause_waits_for_a_chapter_still_running_elsewhere() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    let other = running_elsewhere(&lib).await;
    library::set_queue_paused(&lib, true).unwrap();
    let worker = idle_worker(&lib);
    let settled = queue::run_until_settled(&worker, Scope::All, &|_| {}, &|| {})
        .await
        .unwrap();
    assert_eq!(states(&settled.jobs), [JobState::Done]);
    assert!(settled.paused);
    other.abort();
}
