//! Leases of running jobs: renewal while a page downloads, expiry decided by an injected clock.

mod queue_support;

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use mangas_core::domain::Result;
use mangas_core::download::queue::{
    self, claim, lease_expired, mark_stale, plan_tick, Clock, Job, JobRunner, JobState, Queue,
    Scope, TickInput, LEASE_MS,
};
use mangas_core::download::OnEvent;
use mangas_core::library::{self, Library};
use queue_support::{eventually, fast_config, follow};
use tokio::sync::Notify;

const T: i64 = 1_000_000;

fn running(owner: &str, heartbeat_at: i64) -> Job {
    Job {
        id: 1,
        source: "a".into(),
        pages_done: 3,
        pages_total: 8,
        ..claim(&Job::default(), owner, heartbeat_at)
    }
}

#[test]
fn a_lease_expires_only_past_the_window_and_only_for_running_jobs() {
    let job = running("dead", T);
    assert!(!lease_expired(&job, T + LEASE_MS, LEASE_MS));
    assert!(lease_expired(&job, T + LEASE_MS + 1, LEASE_MS));
    let paused = Job {
        state: JobState::Paused,
        ..job.clone()
    };
    assert!(!lease_expired(&paused, T + 10 * LEASE_MS, LEASE_MS));
    let marked = mark_stale(vec![job.clone(), paused], T + LEASE_MS + 1, LEASE_MS);
    assert_eq!(
        marked.iter().map(|j| j.stale_lease).collect::<Vec<_>>(),
        [true, false]
    );
}

#[test]
fn a_tick_reclaims_another_owners_job_once_its_lease_expired_keeping_its_pages() {
    let config = fast_config();
    let tick = |now: i64| {
        let input = TickInput {
            owner: "me",
            now,
            in_flight: &HashMap::new(),
            finished: &[],
            scope: &Scope::All,
            config: &config,
            paused: false,
        };
        plan_tick(vec![running("dead", T)], &input)
    };
    assert!(tick(T + LEASE_MS).changed.is_empty(), "lease still live");
    let plan = tick(T + LEASE_MS + 1);
    let job = &plan.claimed[0];
    assert_eq!(job.owner.as_deref(), Some("me"));
    assert_eq!(
        (job.pages_done, job.attempts),
        (3, 1),
        "pages kept, attempt not counted twice"
    );
}

#[test]
fn listing_marks_a_running_job_whose_lease_expired() {
    let lib = library::open_in_memory().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    let now = queue::now_ms();
    library::update_jobs(&lib, |jobs| {
        let dead = claim(&jobs[0], "dead", now - LEASE_MS - 1);
        let alive = claim(&jobs[1], "alive", now);
        (vec![dead, alive], ())
    })
    .unwrap();
    let jobs = queue::list(&lib).unwrap();
    assert_eq!((jobs[0].stale_lease, jobs[1].stale_lease), (true, false));
    let json = serde_json::to_value(&jobs[0]).unwrap();
    assert_eq!(json["stale_lease"], true, "the UI gets the flag");
}

/// Starts each job, records the clock reading at the start, then waits for `gate`.
struct GateRunner {
    clock: Clock,
    started_at: std::sync::Mutex<Vec<i64>>,
    gate: Notify,
}

#[async_trait]
impl JobRunner for GateRunner {
    async fn run(&self, _: &Library, _: &Job, _: OnEvent<'_>) -> Result<()> {
        self.started_at.lock().unwrap().push((self.clock)());
        self.gate.notified().await;
        Ok(())
    }
}

fn gate_runner(clock: &Clock) -> Arc<GateRunner> {
    Arc::new(GateRunner {
        clock: clock.clone(),
        started_at: Default::default(),
        gate: Notify::new(),
    })
}

/// A clock that moves `step` ms forward every time it is read.
fn ticking(start: i64, step: i64) -> (Clock, Arc<AtomicI64>) {
    let now = Arc::new(AtomicI64::new(start));
    let read = now.clone();
    (
        Arc::new(move || read.fetch_add(step, Ordering::SeqCst)),
        now,
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn the_owner_renews_the_lease_while_a_page_is_still_downloading() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    // Both "processes" read one clock that runs far faster than real time.
    let (clock, now) = ticking(T, 250);
    let (slow, other) = (gate_runner(&clock), gate_runner(&clock));
    let owner =
        Arc::new(Queue::new(lib.clone(), slow.clone(), fast_config()).with_clock(clock.clone()));
    let rival = Arc::new(Queue::new(lib.clone(), other.clone(), fast_config()).with_clock(clock));
    let a = owner.clone();
    let first = tokio::spawn(async move { a.run(Scope::All, false, &|_| {}).await });
    eventually(|| slow.started_at.lock().unwrap().len() == 1).await;
    let b = rival.clone();
    let second = tokio::spawn(async move { b.run(Scope::All, false, &|_| {}).await });
    // The page never finishes while four lease windows go by on the shared clock.
    eventually(|| now.load(Ordering::SeqCst) > T + 4 * LEASE_MS).await;
    // `list` judges staleness by the wall clock, so read the stored row: who runs it, how often.
    let job = library::list_jobs(&lib).unwrap().remove(0);
    assert_eq!((job.state, job.attempts), (JobState::Running, 1));
    assert_ne!(job.owner, None);
    let age = now.load(Ordering::SeqCst) - job.heartbeat_at;
    assert!(age < LEASE_MS, "renewed lately: {age} {job:?}");
    assert!(
        other.started_at.lock().unwrap().is_empty(),
        "never reclaimed"
    );
    first.abort();
    second.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dead_owners_job_is_reclaimed_only_once_the_injected_clock_passes_the_lease() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    library::update_jobs(&lib, |jobs| (vec![claim(&jobs[0], "dead", T)], ())).unwrap();
    // Exactly at the end of the lease for the first 20 readings, one ms past it afterwards.
    let reads = Arc::new(AtomicUsize::new(0));
    let counted = reads.clone();
    let clock: Clock = Arc::new(move || match counted.fetch_add(1, Ordering::SeqCst) {
        0..20 => T + LEASE_MS,
        _ => T + LEASE_MS + 1,
    });
    let runner = gate_runner(&clock);
    runner.gate.notify_one();
    let worker = Queue::new(lib.clone(), runner.clone(), fast_config()).with_clock(clock);
    worker.run(Scope::All, true, &|_| {}).await.unwrap();
    assert_eq!(*runner.started_at.lock().unwrap(), [T + LEASE_MS + 1]);
    assert!(reads.load(Ordering::SeqCst) > 20);
    let job = queue::list(&lib).unwrap().remove(0);
    assert_eq!((job.state, job.attempts), (JobState::Done, 1));
}
