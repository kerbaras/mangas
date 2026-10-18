mod queue_support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use mangas_core::download::queue::{self, JobState, Queue, Scope};
use mangas_core::library;
use queue_support::{fast_config, follow, states, FakeRunner};

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
