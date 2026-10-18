//! "On disk" means every page is there: a chapter folder that lost pages is queued again on a
//! plain download, which fetches only the missing pages.

mod queue_support;

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use mangas_core::download::queue::{self, JobState, Queue, Scope};
use mangas_core::library;
use queue_support::pages::{pages_on_disk, Fetcher, Runner};
use queue_support::{fast_config, follow};

async fn run(lib: &Arc<library::Library>, fetcher: &Arc<Fetcher>, root: &std::path::Path) {
    let runner = Runner(fetcher.clone(), root.to_path_buf());
    let worker = Queue::new(lib.clone(), Arc::new(runner), fast_config());
    worker.run(Scope::All, true, &|_| {}).await.unwrap();
}

/// A library following one six-page chapter, downloaded once into `root`; returns its folder.
async fn downloaded(
    root: &std::path::Path,
) -> (
    Arc<library::Library>,
    Vec<mangas_core::domain::Chapter>,
    PathBuf,
) {
    let lib = Arc::new(library::open_in_memory().unwrap());
    let chapters = follow(&lib, "a", 1);
    queue::enqueue(&lib, "a", chapters.clone(), false).unwrap();
    run(&lib, &Arc::new(Fetcher::default()), root).await;
    let dir = queue::list(&lib).unwrap()[0]
        .local_path
        .clone()
        .unwrap()
        .into();
    (lib, chapters, dir)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_partial_folder_fetches_only_its_missing_pages() {
    let root = tempfile::tempdir().unwrap();
    let (lib, chapters, dir) = downloaded(root.path()).await;
    std::fs::write(dir.join("0001.jpg"), b"kept").unwrap();
    std::fs::write(dir.join("0001.png"), b"an extra image").unwrap();
    std::fs::remove_file(dir.join("0002.jpg")).unwrap();
    std::fs::remove_file(dir.join("0005.jpg")).unwrap();

    let queued = queue::enqueue(&lib, "a", chapters, false).unwrap();
    assert_eq!((queued.jobs.len(), queued.on_disk.len()), (1, 0));
    let fetcher = Arc::new(Fetcher::default());
    run(&lib, &fetcher, root.path()).await;

    let mut fetched = fetcher.fetched.lock().unwrap().clone();
    fetched.sort();
    assert_eq!(fetched, ["https://cdn/2", "https://cdn/5"]);
    assert_eq!(std::fs::read(dir.join("0001.jpg")).unwrap(), b"kept");
    let job = queue::list(&lib).unwrap().remove(0);
    assert_eq!(
        (job.state, job.force, job.pages_total),
        (JobState::Done, false, 6)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_complete_folder_is_still_skipped() {
    let root = tempfile::tempdir().unwrap();
    let (lib, chapters, dir) = downloaded(root.path()).await;
    assert_eq!(
        std::fs::read_to_string(dir.join(".mangas-pages")).unwrap(),
        "6\n"
    );

    let queued = queue::enqueue(&lib, "a", chapters, false).unwrap();
    assert_eq!((queued.jobs.len(), queued.on_disk.len()), (0, 1));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_from_before_the_marker_counts_when_it_has_pages() {
    let root = tempfile::tempdir().unwrap();
    let (lib, chapters, dir) = downloaded(root.path()).await;
    std::fs::remove_file(dir.join(".mangas-pages")).unwrap();
    std::fs::remove_file(dir.join("0002.jpg")).unwrap();
    assert_eq!(
        queue::enqueue(&lib, "a", chapters.clone(), false)
            .unwrap()
            .on_disk
            .len(),
        1
    );

    for page in 1..=6 {
        let _ = std::fs::remove_file(dir.join(format!("{page:04}.jpg")));
    }
    let queued = queue::enqueue(&lib, "a", chapters, false).unwrap();
    assert_eq!(queued.jobs.len(), 1, "an emptied folder downloads again");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_page_count_is_recorded_before_the_pages_arrive() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    let root = tempfile::tempdir().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    let fetcher = Arc::new(Fetcher::default());
    fetcher.hang.store(true, Ordering::SeqCst);
    let worker = {
        let (lib, fetcher, root) = (lib.clone(), fetcher.clone(), root.path().to_path_buf());
        tokio::spawn(async move { run(&lib, &fetcher, &root).await })
    };
    for _ in 0..500 {
        if pages_on_disk(root.path()).len() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    worker.abort();

    assert_eq!(
        find_marker(root.path()).as_deref(),
        Some("6\n"),
        "a run that dies mid-way leaves the full count"
    );
}

fn find_marker(dir: &std::path::Path) -> Option<String> {
    let marker = dir.join(".mangas-pages");
    if marker.exists() {
        return std::fs::read_to_string(marker).ok();
    }
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .find_map(|p| find_marker(&p))
}
