mod queue_support;

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mangas_core::domain::Error;
use mangas_core::download::queue::{self, Action, JobState, Queue, QueueEvent, Scope, Target};
use mangas_core::library;
use queue_support::{eventually, fast_config, follow, states, FakeRunner};

fn runner(delay_ms: u64) -> FakeRunner {
    FakeRunner {
        delay: Mutex::new(Duration::from_millis(delay_ms)),
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn runs_every_job_concurrently_within_the_caps() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    let a = follow(&lib, "a", 5);
    let b = follow(&lib, "b", 3);
    queue::enqueue(&lib, "a", a, false).unwrap();
    queue::enqueue(&lib, "b", b, false).unwrap();
    let fake = Arc::new(runner(80));
    let worker = Queue::new(lib.clone(), fake.clone(), fast_config());
    let events = Mutex::new(Vec::new());
    let sink = |e: QueueEvent| events.lock().unwrap().push(e);
    worker.run(Scope::All, true, &sink).await.unwrap();

    let jobs = queue::list(&lib).unwrap();
    assert_eq!(states(&jobs), [JobState::Done; 8]);
    assert!(jobs.iter().all(|j| j.pages_done == 4 && j.pages_total == 4));
    let max = fake.max_per_source.lock().unwrap().clone();
    assert_eq!(max["a"], 2, "per-provider cap of 2 used");
    assert!(max["b"] <= 2, "per-provider cap never passed: {max:?}");
    assert_eq!(fake.max_total.load(Ordering::SeqCst), 3, "global cap of 3");
    let events = events.lock().unwrap();
    let count = |f: fn(&QueueEvent) -> bool| events.iter().filter(|e| f(e)).count();
    assert_eq!(count(|e| matches!(e, QueueEvent::Done { .. })), 8);
    assert!(count(|e| matches!(e, QueueEvent::Started { .. })) >= 1);
    assert!(count(|e| matches!(e, QueueEvent::Progress { .. })) >= 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_transient_failure_is_retried_after_a_backoff() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    let fake = Arc::new(FakeRunner {
        fail_first: 2,
        error: Some(|| Error::Other("incomplete chapter: 1 of 4 pages failed".into())),
        ..runner(4)
    });
    let worker = Queue::new(lib.clone(), fake.clone(), fast_config());
    let events = Mutex::new(Vec::new());
    let sink = |e: QueueEvent| events.lock().unwrap().push(e);
    worker.run(Scope::All, true, &sink).await.unwrap();

    let job = &queue::list(&lib).unwrap()[0];
    assert_eq!(
        (job.state, job.attempts, job.error.as_deref()),
        (JobState::Done, 3, None)
    );
    let retries = events
        .lock()
        .unwrap()
        .iter()
        .filter(|e| matches!(e, QueueEvent::Queued { job } if job.error.is_some()))
        .count();
    assert_eq!(retries, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_challenge_fails_at_once_and_retry_queues_it_again() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    let fake = Arc::new(FakeRunner {
        fail_first: 1,
        error: Some(|| Error::Challenge {
            name: "MangaDot".into(),
            site: "https://mangadot.net".into(),
        }),
        ..runner(4)
    });
    let worker = Queue::new(lib.clone(), fake, fast_config());
    worker.run(Scope::All, true, &|_| {}).await.unwrap();
    let job = queue::list(&lib).unwrap().remove(0);
    assert_eq!((job.state, job.attempts), (JobState::Failed, 1));
    assert!(job.error.unwrap().starts_with("challenge:"));

    queue::control(&lib, Target::Job(job.id), Action::Retry).unwrap();
    worker.run(Scope::All, true, &|_| {}).await.unwrap();
    assert_eq!(queue::list(&lib).unwrap()[0].state, JobState::Done);
}

#[tokio::test(flavor = "multi_thread")]
async fn pause_stops_a_running_download_and_resume_finishes_it() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    let fake = Arc::new(runner(60_000));
    let worker = Arc::new(Queue::new(lib.clone(), fake.clone(), fast_config()));
    let background = worker.clone();
    let task = tokio::spawn(async move { background.run(Scope::All, false, &|_| {}).await });
    let state = || queue::list(&lib).unwrap()[0].state;
    eventually(|| state() == JobState::Running).await;

    queue::control(&lib, Target::All, Action::Pause).unwrap();
    eventually(|| fake.aborted.load(Ordering::SeqCst)).await;
    assert_eq!(state(), JobState::Paused);

    *fake.delay.lock().unwrap() = Duration::from_millis(4);
    queue::control(&lib, Target::All, Action::Resume).unwrap();
    eventually(|| state() == JobState::Done).await;
    task.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pause_and_resume_between_ticks_never_runs_the_chapter_twice() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    let fake = Arc::new(runner(60_000));
    let config = queue::QueueConfig {
        tick: Duration::from_secs(3600),
        ..fast_config()
    };
    let worker = Arc::new(Queue::new(lib.clone(), fake.clone(), config));
    let background = worker.clone();
    let task = tokio::spawn(async move { background.run(Scope::All, false, &|_| {}).await });
    let state = || queue::list(&lib).unwrap()[0].state;
    eventually(|| state() == JobState::Running).await;

    // No nudge in between: the worker sees the job queued again, never paused.
    queue::control(&lib, Target::All, Action::Pause).unwrap();
    *fake.delay.lock().unwrap() = Duration::from_millis(40);
    queue::control(&lib, Target::All, Action::Resume).unwrap();
    worker.nudge();
    eventually(|| state() == JobState::Done).await;
    assert!(
        fake.aborted.load(Ordering::SeqCst),
        "the first run was stopped"
    );
    assert_eq!(
        fake.max_per_source.lock().unwrap()["a"],
        1,
        "never two runs at once"
    );
    task.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_stops_a_running_download_for_good() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    let fake = Arc::new(runner(60_000));
    let worker = Arc::new(Queue::new(lib.clone(), fake.clone(), fast_config()));
    let background = worker.clone();
    let task = tokio::spawn(async move { background.run(Scope::All, false, &|_| {}).await });
    eventually(|| queue::list(&lib).unwrap()[0].state == JobState::Running).await;
    let id = queue::list(&lib).unwrap()[0].id;
    queue::control(&lib, Target::Job(id), Action::Cancel).unwrap();
    eventually(|| fake.aborted.load(Ordering::SeqCst)).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let job = queue::list(&lib).unwrap().remove(0);
    assert_eq!((job.state, job.owner), (JobState::Cancelled, None));
    task.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_scoped_worker_leaves_other_jobs_alone() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    let queued = queue::enqueue(&lib, "a", follow(&lib, "a", 3), false).unwrap();
    let mine = Scope::Jobs([queued.jobs[1]].into());
    let worker = Queue::new(lib.clone(), Arc::new(runner(4)), fast_config());
    worker.run(mine, true, &|_| {}).await.unwrap();
    let jobs = queue::list(&lib).unwrap();
    assert_eq!(
        states(&jobs),
        [JobState::Queued, JobState::Done, JobState::Queued]
    );
}
