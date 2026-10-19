//! "Download again" (`force`): every page is fetched again and overwrites the file on disk.

mod queue_support;

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

fn chapter_dir(lib: &library::Library) -> std::path::PathBuf {
    queue::list(lib).unwrap()[0]
        .local_path
        .clone()
        .unwrap()
        .into()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_forced_download_rewrites_every_page_even_an_emptied_one() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    let root = tempfile::tempdir().unwrap();
    let chapters = follow(&lib, "a", 1);
    queue::enqueue(&lib, "a", chapters.clone(), false).unwrap();
    run(&lib, &Arc::new(Fetcher::default()), root.path()).await;
    let dir = chapter_dir(&lib);
    std::fs::write(dir.join("0001.jpg"), b"old").unwrap();
    std::fs::write(dir.join("0003.jpg"), b"").unwrap();
    std::fs::write(dir.join("0001.png"), b"other extension").unwrap();
    std::fs::write(dir.join("0009.jpg"), b"dropped page").unwrap();

    let skipped = queue::enqueue(&lib, "a", chapters.clone(), false).unwrap();
    assert_eq!(skipped.on_disk.len(), 1);
    let forced = queue::enqueue(&lib, "a", chapters, true).unwrap();
    assert_eq!(forced.jobs.len(), 1);
    assert!(queue::list(&lib).unwrap()[0].force);
    let fetcher = Arc::new(Fetcher::default());
    run(&lib, &fetcher, root.path()).await;

    assert_eq!(
        fetcher.fetched.lock().unwrap().len(),
        6,
        "every page fetched again"
    );
    assert_eq!(
        std::fs::read(dir.join("0001.jpg")).unwrap(),
        b"https://cdn/1"
    );
    assert_eq!(
        std::fs::read(dir.join("0003.jpg")).unwrap(),
        b"https://cdn/3"
    );
    assert!(!dir.join("0001.png").exists(), "stale extension removed");
    assert!(!dir.join("0009.jpg").exists(), "dropped page removed");
    let job = queue::list(&lib).unwrap().remove(0);
    assert_eq!(
        (job.state, job.pages_done, job.pages_total),
        (JobState::Done, 6, 6)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn without_force_a_chapter_on_disk_is_left_alone() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    let root = tempfile::tempdir().unwrap();
    let chapters = follow(&lib, "a", 1);
    queue::enqueue(&lib, "a", chapters.clone(), false).unwrap();
    run(&lib, &Arc::new(Fetcher::default()), root.path()).await;
    let dir = chapter_dir(&lib);
    std::fs::write(dir.join("0001.jpg"), b"old").unwrap();

    let skipped = queue::enqueue(&lib, "a", chapters, false).unwrap();
    assert_eq!((skipped.jobs.len(), skipped.on_disk.len()), (0, 1));
    assert!(!queue::list(&lib).unwrap()[0].force);
    let fetcher = Arc::new(Fetcher::default());
    run(&lib, &fetcher, root.path()).await;

    assert!(fetcher.fetched.lock().unwrap().is_empty());
    assert_eq!(std::fs::read(dir.join("0001.jpg")).unwrap(), b"old");
}

#[test]
fn a_forced_request_on_a_running_job_stays_with_its_owner() {
    let run = queue::Job {
        id: 1,
        state: JobState::Running,
        owner: Some("me".into()),
        created_at: 5,
        ..queue::Job::default()
    };
    let fresh = queue::Job {
        force: true,
        ..queue::Job::default()
    };
    let again = queue::enqueue_job(Some(&run), fresh.clone(), 1_000).unwrap();
    assert_eq!(
        (again.state, again.force, again.owner, again.created_at),
        (JobState::Running, true, Some("me".into()), 5)
    );
    let forced = queue::Job { force: true, ..run };
    assert!(queue::enqueue_job(Some(&forced), fresh, 1_000).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn forcing_a_running_chapter_restarts_it_as_a_full_download() {
    let lib = Arc::new(library::open_in_memory().unwrap());
    let root = tempfile::tempdir().unwrap();
    let chapters = follow(&lib, "a", 1);
    let fetcher = Arc::new(Fetcher::default());
    fetcher.hang.store(true, Ordering::SeqCst);
    queue::enqueue(&lib, "a", chapters.clone(), false).unwrap();
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

    // The run in flight hangs on page 3 for good: only its owner dropping it lets this finish.
    queue::enqueue(&lib, "a", chapters, true).unwrap();
    fetcher.hang.store(false, Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(10), worker)
        .await
        .expect("the forced run replaced the hung one")
        .unwrap();

    let job = queue::list(&lib).unwrap().remove(0);
    assert_eq!((job.state, job.force), (JobState::Done, true));
    let fetched = fetcher.fetched.lock().unwrap().clone();
    let first = fetched.iter().filter(|u| *u == "https://cdn/1").count();
    assert_eq!((first, fetched.len()), (2, 8), "{fetched:?}");
}

#[test]
fn imported_chapters_are_never_queued_even_forced() {
    let lib = library::open_in_memory().unwrap();
    let chapters = follow(&lib, library::LOCAL_SOURCE, 1);
    let err = queue::enqueue(&lib, library::LOCAL_SOURCE, chapters, true).unwrap_err();
    assert!(err.to_string().contains("already on disk"), "{err}");
    assert!(queue::list(&lib).unwrap().is_empty());
    assert!(queue::ensure_downloadable("mangadex").is_ok());
}
