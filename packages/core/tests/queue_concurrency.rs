mod queue_support;

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mangas_core::download::queue::{self, Limits, Queue, Scope};
use mangas_core::library::{self, Settings};
use queue_support::{eventually, fast_config, follow, started, FakeRunner};

#[test]
fn concurrency_maps_to_overall_and_per_provider_limits() {
    let limits = |n| {
        let l = Limits::for_concurrency(n);
        (l.global, l.per_provider)
    };
    assert_eq!(limits(1), (1, 1));
    assert_eq!(limits(2), (2, 2));
    assert_eq!(limits(4), (4, 2), "the default keeps today's limits");
    assert_eq!(limits(5), (5, 3));
    assert_eq!(limits(8), (8, 4));
}

fn set_concurrency(lib: &library::Library, n: u8) {
    let settings = Settings {
        download_concurrency: Some(n),
        ..library::get_settings(lib).unwrap()
    };
    library::set_settings(lib, &settings).unwrap();
}

fn running(fake: &FakeRunner) -> usize {
    fake.running.lock().unwrap().values().sum()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_concurrency_change_applies_on_the_next_tick() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 4), false).unwrap();
    queue::enqueue(&lib, "b", follow(&lib, "b", 4), false).unwrap();
    set_concurrency(&lib, 1);
    let fake = Arc::new(FakeRunner {
        delay: Mutex::new(Duration::from_millis(400)),
        ..Default::default()
    });
    let worker =
        Arc::new(Queue::new(lib.clone(), fake.clone(), fast_config()).following_settings());
    let background = worker.clone();
    let task = tokio::spawn(async move { background.run(Scope::All, false, &|_| {}).await });

    started(&fake, 1).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(fake.max_total.load(Ordering::SeqCst), 1, "one at a time");

    // No restart, no nudge: the worker's next step reads the new setting.
    set_concurrency(&lib, 3);
    eventually(|| running(&fake) == 3).await;
    assert_eq!(fake.max_total.load(Ordering::SeqCst), 3);
    assert!(fake
        .max_per_source
        .lock()
        .unwrap()
        .values()
        .all(|&n| n <= 2));
    task.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_worker_not_following_settings_keeps_its_limits() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    queue::enqueue(&lib, "a", follow(&lib, "a", 3), false).unwrap();
    set_concurrency(&lib, 1);
    let fake = Arc::new(FakeRunner {
        delay: Mutex::new(Duration::from_millis(60)),
        ..Default::default()
    });
    let worker = Queue::new(lib.clone(), fake.clone(), fast_config());
    worker.run(Scope::All, true, &|_| {}).await.unwrap();
    assert_eq!(
        fake.max_total.load(Ordering::SeqCst),
        2,
        "fast_config's per-provider cap"
    );
}
