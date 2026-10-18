//! Storage-move holds and the CLI's settling run through them, against a fake runner.

mod queue_support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mangas_core::download::queue::{self, Action, JobState, Queue, QueueGate, Scope, Target};
use mangas_core::library;
use queue_support::{eventually, fast_config, follow, states, FakeRunner};

fn runner(delay_ms: u64) -> FakeRunner {
    FakeRunner {
        delay: Mutex::new(Duration::from_millis(delay_ms)),
        ..Default::default()
    }
}

fn one_at_a_time() -> queue::QueueConfig {
    queue::QueueConfig {
        limits: queue::Limits {
            per_provider: 1,
            global: 1,
        },
        ..fast_config()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pause_before_the_start_ends_the_run_with_every_chapter_queued() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    // Pausing all pauses each job too; resuming them one by one leaves only the queue paused.
    queue::control(&lib, Target::All, Action::Pause).unwrap();
    queue::control(&lib, Target::Job(1), Action::Resume).unwrap();
    queue::control(&lib, Target::Job(2), Action::Resume).unwrap();
    let worker = Queue::new(lib.clone(), Arc::new(runner(4)), fast_config());
    let settled = queue::run_until_settled(&worker, Scope::All, &|_| {}, &|| {})
        .await
        .unwrap();
    assert!(settled.paused);
    assert_eq!(states(&settled.jobs), [JobState::Queued; 2]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pause_mid_run_lets_the_running_chapter_finish_and_reports_the_rest() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    let worker = Arc::new(Queue::new(
        lib.clone(),
        Arc::new(runner(400)),
        one_at_a_time(),
    ));
    let background = worker.clone();
    let run = tokio::spawn(async move {
        queue::run_until_settled(&background, Scope::All, &|_| {}, &|| {}).await
    });
    eventually(|| queue::list(&lib).unwrap()[0].state == JobState::Running).await;
    library::set_queue_paused(&lib, true).unwrap();
    let settled = run.await.unwrap().unwrap();
    assert!(settled.paused);
    assert_eq!(states(&settled.jobs), [JobState::Done, JobState::Queued]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_storage_move_hold_is_waited_out_and_the_chapters_carry_on() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    let worker = Arc::new(Queue::new(
        lib.clone(),
        Arc::new(runner(300)),
        one_at_a_time(),
    ));
    let holds = Arc::new(AtomicUsize::new(0));
    let (background, seen) = (worker.clone(), holds.clone());
    let run = tokio::spawn(async move {
        let on_hold = move || {
            seen.fetch_add(1, Ordering::SeqCst);
        };
        queue::run_until_settled(&background, Scope::All, &|_| {}, &on_hold).await
    });
    eventually(|| queue::list(&lib).unwrap()[0].state == JobState::Running).await;
    let held = lib.clone();
    tokio::task::spawn_blocking(move || {
        queue::hold_queue(&held, Duration::from_secs(10), || {
            // "Resume all" from another terminal can't end a move's hold.
            queue::control(&held, Target::All, Action::Resume)?;
            std::thread::sleep(Duration::from_millis(1500));
            assert!(library::queue_gate(&held)?.held(queue::now_ms()));
            Ok(())
        })
    })
    .await
    .unwrap()
    .unwrap();
    let settled = run.await.unwrap().unwrap();
    assert!(!settled.paused);
    assert_eq!(states(&settled.jobs), [JobState::Done; 2]);
    assert_eq!(holds.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_held_queue_lets_the_running_job_finish_before_the_move_and_loses_nothing() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    let config = queue::QueueConfig {
        limits: queue::Limits {
            per_provider: 1,
            global: 1,
        },
        ..fast_config()
    };
    let worker = Arc::new(Queue::new(lib.clone(), Arc::new(runner(400)), config));
    let background = worker.clone();
    let task = tokio::spawn(async move { background.run(Scope::All, false, &|_| {}).await });
    eventually(|| queue::list(&lib).unwrap()[0].state == JobState::Running).await;
    let held = lib.clone();
    let seen = tokio::task::spawn_blocking(move || {
        queue::hold_queue(&held, Duration::from_secs(10), || {
            Ok(states(&queue::list(&held)?))
        })
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        seen,
        [JobState::Done, JobState::Queued],
        "nothing ran during the move"
    );
    assert!(!library::queue_paused(&lib).unwrap());
    eventually(|| states(&queue::list(&lib).unwrap()) == [JobState::Done; 2]).await;
    task.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_hold_that_does_not_settle_refuses_and_releases_the_queue() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    let worker = Arc::new(Queue::new(
        lib.clone(),
        Arc::new(runner(60_000)),
        fast_config(),
    ));
    let background = worker.clone();
    let task = tokio::spawn(async move { background.run(Scope::All, false, &|_| {}).await });
    eventually(|| queue::list(&lib).unwrap()[0].state == JobState::Running).await;
    let held = lib.clone();
    let err = tokio::task::spawn_blocking(move || {
        queue::hold_queue(&held, Duration::from_millis(300), || Ok(()))
    })
    .await
    .unwrap()
    .unwrap_err();
    assert!(
        err.to_string().starts_with("1 download(s) still running"),
        "{err}"
    );
    assert_eq!(library::queue_gate(&lib).unwrap(), QueueGate::default());
    task.abort();
}
