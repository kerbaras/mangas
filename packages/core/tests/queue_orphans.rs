//! A worker process that died mid-chapter (kill -9): its job neither blocks a storage move nor
//! loses the pages it wrote.

mod queue_support;

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use mangas_core::download::queue::{self, claim, Clock, Job, JobState, Queue, Scope, LEASE_MS};
use mangas_core::library::{self, Library};
use queue_support::pages::{pages_on_disk, Fetcher, Runner};
use queue_support::{fast_config, follow};

/// One chapter queued, then claimed by a process that last renewed its lease at `heartbeat`.
fn left_running(heartbeat: i64) -> Library {
    let lib = library::open_in_memory().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    library::update_jobs(&lib, |jobs| {
        let job = Job {
            pages_done: 3,
            pages_total: 8,
            ..claim(&jobs[0], "dead", heartbeat)
        };
        (vec![job], ())
    })
    .unwrap();
    lib
}

#[test]
fn a_storage_move_goes_ahead_past_a_dead_processs_lease_and_requeues_its_job() {
    let lib = left_running(queue::now_ms() - LEASE_MS - 1);
    let dir = tempfile::tempdir().unwrap();
    let (from, to) = (dir.path().join("old"), dir.path().join("new"));
    std::fs::create_dir_all(&from).unwrap();
    mangas_core::storage::move_data_dir(&lib, &from, &to, &mut |_| {}).unwrap();
    let job = queue::list(&lib).unwrap().remove(0);
    assert_eq!(
        (job.state, job.pages_done, job.attempts),
        (JobState::Queued, 3, 0)
    );
    assert_eq!(job.owner, None);
}

#[test]
fn within_the_lease_a_dead_processs_job_still_counts_as_running() {
    let lib = left_running(queue::now_ms());
    let err = queue::hold_queue(&lib, Duration::ZERO, || Ok(())).unwrap_err();
    assert!(
        err.to_string().starts_with("1 download(s) still running"),
        "{err}"
    );
    let job = queue::list(&lib).unwrap().remove(0);
    assert_eq!((job.state, job.stale_lease), (JobState::Running, false));
}

#[test]
fn a_move_started_inside_the_lease_waits_for_it_to_run_out_then_goes_ahead() {
    const T: i64 = 1_000_000;
    let lib = left_running(T);
    // Each reading of the move's clock is one second later.
    let now = Arc::new(AtomicI64::new(T));
    let read = now.clone();
    let clock: Clock = Arc::new(move || read.fetch_add(1_000, Ordering::SeqCst));
    let seen = queue::hold_queue_at(&lib, queue::MOVE_SETTLE, &clock, || {
        Ok(queue::list(&lib)?.remove(0).state)
    })
    .unwrap();
    assert_eq!(seen, JobState::Queued, "requeued before the move ran");
    assert!(now.load(Ordering::SeqCst) > T + LEASE_MS);
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
}

#[test]
fn a_worker_killed_mid_chapter_loses_no_pages_and_its_job_resumes_on_the_next_start() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    let root = tempfile::tempdir().unwrap();
    let fetcher = Arc::new(Fetcher::default());
    fetcher.hang.store(true, Ordering::SeqCst);
    let runner = Arc::new(Runner(fetcher.clone(), root.path().to_path_buf()));

    // "kill -9": the process's runtime vanishes mid-chapter, nothing gets to clean up.
    let doomed = runtime();
    let worker = Arc::new(Queue::new(lib.clone(), runner.clone(), fast_config()));
    doomed.spawn(async move { worker.run(Scope::All, false, &|_| {}).await });
    for _ in 0..500 {
        if pages_on_disk(root.path()).len() == 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    doomed.shutdown_background();
    let kept = pages_on_disk(root.path());
    assert_eq!(kept.len(), 2, "{kept:?}");
    assert_eq!(queue::list(&lib).unwrap()[0].state, JobState::Running);

    // The next process starts once the lease ran out: the job goes back to the queue.
    let later = queue::now_ms() + LEASE_MS + 1;
    let back = queue::reclaim_orphans(&lib, later).unwrap();
    assert_eq!(
        (back.len(), back[0].state, back[0].attempts),
        (1, JobState::Queued, 0)
    );
    fetcher.hang.store(false, Ordering::SeqCst);
    fetcher.fetched.lock().unwrap().clear();
    let worker = Queue::new(lib.clone(), runner, fast_config());
    runtime()
        .block_on(worker.run(Scope::All, true, &|_| {}))
        .unwrap();

    let job = queue::list(&lib).unwrap().remove(0);
    assert_eq!((job.state, job.attempts), (JobState::Done, 1));
    let all = pages_on_disk(root.path());
    assert_eq!(all.len(), 6);
    assert!(kept.is_subset(&all));
    let refetched = fetcher.fetched.lock().unwrap().clone();
    assert_eq!(refetched.len(), 4, "only the missing pages: {refetched:?}");
}
