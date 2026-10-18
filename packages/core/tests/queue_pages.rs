mod queue_support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use mangas_core::download::queue::{self, JobState, Queue, QueueConfig, QueueEvent, Scope};
use mangas_core::library;
use queue_support::{eventually, fast_config, follow, states, FakeRunner};

fn runner(delay_ms: u64) -> FakeRunner {
    FakeRunner {
        delay: Mutex::new(Duration::from_millis(delay_ms)),
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_chapter_done_between_two_ticks_keeps_its_page_count() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    let worker = Queue::new(lib.clone(), Arc::new(runner(0)), fast_config());
    worker.run(Scope::All, true, &|_| {}).await.unwrap();

    let jobs = queue::list(&lib).unwrap();
    assert_eq!(states(&jobs), [JobState::Done]);
    assert_eq!((jobs[0].pages_done, jobs[0].pages_total), (4, 4));
}

#[tokio::test(flavor = "multi_thread")]
async fn every_page_landed_is_saved_without_waiting_for_a_tick() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    // A page every 1.5 s, and no tick for a minute: only a landed page wakes the worker.
    let config = QueueConfig {
        tick: Duration::from_secs(60),
        ..fast_config()
    };
    let worker = Arc::new(Queue::new(lib.clone(), Arc::new(runner(6_000)), config));
    let events = Arc::new(Mutex::new(Vec::new()));
    let (w, sink) = (worker.clone(), events.clone());
    let run = tokio::spawn(async move {
        let on_event = move |e: QueueEvent| sink.lock().unwrap().push(e);
        w.run(Scope::All, true, &on_event).await.unwrap()
    });

    let job = || queue::list(&lib).unwrap()[0].clone();
    eventually(|| job().pages_done == 2).await;
    assert_eq!((job().state, job().pages_total), (JobState::Running, 4));
    let reported =
        |e: &QueueEvent| matches!(e, QueueEvent::Progress { job } if job.pages_done == 2);
    assert!(
        events.lock().unwrap().iter().any(reported),
        "the Downloads screen hears it too"
    );
    run.abort();
}
