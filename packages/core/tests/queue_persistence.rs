mod queue_support;

use std::collections::HashSet;
use std::sync::Arc;

use mangas_core::domain::{Availability, MangaId};
use mangas_core::download::plan;
use mangas_core::download::queue::{self, claim, Action, JobState, Queue, Scope, Target};
use mangas_core::library;
use mangas_core::sources::PageUrls;
use queue_support::{chapter, fast_config, follow, states, FakeRunner};

#[tokio::test(flavor = "multi_thread")]
async fn pending_jobs_survive_a_restart_and_a_crashed_run_resumes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mangas.db");
    {
        let lib = library::open(&path).unwrap();
        queue::enqueue(&lib, "a", follow(&lib, "a", 3), false).unwrap();
        queue::control(&lib, Target::Job(3), Action::Pause).unwrap();
        // Job 1 was running in a process that died: its heartbeat is long gone.
        library::update_jobs(&lib, |jobs| (vec![claim(&jobs[0], "dead-process", 0)], ())).unwrap();
    }
    let lib = Arc::new(library::open(&path).unwrap());
    let before = queue::list(&lib).unwrap();
    assert_eq!(
        states(&before),
        [JobState::Running, JobState::Queued, JobState::Paused]
    );
    let config = queue::QueueConfig {
        stale_after_ms: 50,
        ..fast_config()
    };
    let worker = Queue::new(lib.clone(), Arc::new(FakeRunner::default()), config);
    worker.run(Scope::All, true, &|_| {}).await.unwrap();
    let after = queue::list(&lib).unwrap();
    assert_eq!(
        states(&after),
        [JobState::Done, JobState::Done, JobState::Paused]
    );
    assert_eq!(
        after[0].attempts, 1,
        "the interrupted attempt is not counted"
    );
}

#[test]
fn enqueue_skips_chapters_on_disk_unless_forced_and_reports_unavailable_ones() {
    let lib = library::open_in_memory().unwrap();
    let mut chapters = follow(&lib, "a", 3);
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("0001.jpg"), b"page").unwrap();
    std::fs::write(dir.path().join(".mangas-pages"), "1\n").unwrap();
    library::mark_downloaded(&lib, "a", &chapters[0].id, dir.path()).unwrap();
    chapters[0].local_path = Some(dir.path().to_string_lossy().into_owned());
    chapters[2].availability = Availability::Locked;

    let queued = queue::enqueue(&lib, "a", chapters.clone(), false).unwrap();
    assert_eq!(queued.jobs.len(), 1);
    assert_eq!(queued.on_disk[0].id, chapters[0].id);
    assert_eq!(queued.unavailable[0].id, chapters[2].id);

    // A list fresh from the source has no local paths; the library still knows.
    let fresh: Vec<_> = chapters
        .iter()
        .map(|c| chapter(&c.id.0, &c.number))
        .collect();
    let again = queue::enqueue(&lib, "a", fresh[..1].to_vec(), false).unwrap();
    assert_eq!((again.jobs.len(), again.on_disk.len()), (0, 1));

    let forced = queue::enqueue(&lib, "a", chapters, true).unwrap();
    assert_eq!(
        forced.jobs.len(),
        2,
        "the downloaded one plus the still active one"
    );
    assert!(
        forced.jobs.contains(&queued.jobs[0]),
        "an active job is kept, not duplicated"
    );
    assert_eq!(queue::list(&lib).unwrap().len(), 2);
}

#[test]
fn jobs_carry_their_chapter_and_manga_for_display() {
    let lib = library::open_in_memory().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    let job = queue::list(&lib).unwrap().remove(0);
    assert_eq!(
        (job.manga_title.as_str(), job.number.as_str()),
        ("Kaiju", "1")
    );
    assert_eq!(job.manga_id, MangaId("m".into()));
    assert_eq!(job.language, "en");
}

#[test]
fn control_explains_a_refusal_and_clear_removes_finished_jobs() {
    let lib = library::open_in_memory().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    let err = queue::control(&lib, Target::Job(1), Action::Resume).unwrap_err();
    assert_eq!(err.to_string(), "download 1 is queued; nothing to resume");
    let err = queue::control(&lib, Target::Job(99), Action::Pause).unwrap_err();
    assert_eq!(err.to_string(), "not found: download 99");

    queue::control(&lib, Target::Job(2), Action::Cancel).unwrap();
    assert_eq!(queue::clear_finished(&lib).unwrap(), 1);
    assert_eq!(states(&queue::list(&lib).unwrap()), [JobState::Queued]);
    assert!(queue::control(&lib, Target::All, Action::Retry)
        .unwrap()
        .is_empty());
}

#[test]
fn unfollowing_a_manga_drops_its_jobs() {
    let lib = library::open_in_memory().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    library::remove_manga(&lib, "a", &MangaId("m".into())).unwrap();
    assert!(queue::list(&lib).unwrap().is_empty());
}

#[test]
fn a_refresh_fails_the_job_of_a_dropped_chapter_instead_of_dropping_it() {
    let lib = library::open_in_memory().unwrap();
    let chapters = follow(&lib, "a", 3);
    queue::enqueue(&lib, "a", chapters.clone(), false).unwrap();
    queue::control(&lib, Target::Job(3), Action::Cancel).unwrap();
    // The source now lists chapter 1 only.
    let manga = MangaId("m".into());
    library::upsert_chapters(&lib, "a", &manga, "en", &chapters[..1]).unwrap();

    let jobs = queue::list(&lib).unwrap();
    assert_eq!(states(&jobs), [JobState::Queued, JobState::Failed]);
    assert_eq!(jobs[1].error.as_deref(), Some(queue::UNLISTED));
    let kept: Vec<String> = library::list_chapters(&lib, "a", &manga, None)
        .unwrap()
        .into_iter()
        .map(|c| c.id.0)
        .collect();
    assert_eq!(
        kept,
        ["a-1", "a-2"],
        "the cancelled one goes, the failed one stays"
    );

    // Once the user dismisses and clears it, the next refresh drops the row as before.
    queue::control(&lib, Target::Job(2), Action::Cancel).unwrap();
    queue::clear_finished(&lib).unwrap();
    library::upsert_chapters(&lib, "a", &manga, "en", &chapters[..1]).unwrap();
    assert_eq!(
        library::list_chapters(&lib, "a", &manga, None)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn storage_move_holds_the_queue_and_restores_the_pause_flag() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::open_in_memory().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 1), false).unwrap();
    let (from, to) = (dir.path().join("old"), dir.path().join("new"));
    std::fs::create_dir_all(&from).unwrap();
    mangas_core::storage::move_data_dir(&lib, &from, &to, &mut |_| {}).unwrap();
    assert!(
        !library::queue_paused(&lib).unwrap(),
        "restored after a move"
    );
    library::set_queue_paused(&lib, true).unwrap();
    queue::hold_queue(&lib, std::time::Duration::from_secs(5), || Ok(())).unwrap();
    assert!(
        library::queue_paused(&lib).unwrap(),
        "a pause set before stays set"
    );
}

#[test]
fn a_page_on_disk_counts_by_index_whatever_its_extension() {
    // A full-quality run left 0001.png; with data saver the same page is now a .jpg URL.
    let urls = PageUrls(vec!["https://x/1-a.jpg".into(), "https://x/2-b.jpg".into()]);
    let existing: HashSet<String> = ["0001.png", "0002.jpg.part", "0003.txt"]
        .map(String::from)
        .into();
    let urls = PageUrls([urls.0, vec!["https://x/3-c.jpg".into()]].concat());
    let jobs = plan(&urls, &existing);
    assert_eq!(jobs.iter().map(|j| j.index).collect::<Vec<_>>(), [2, 3]);
}

#[test]
fn a_chapter_whose_folder_was_deleted_outside_the_app_is_downloaded_again() {
    let lib = library::open_in_memory().unwrap();
    let chapters = follow(&lib, "a", 2);
    let dir = tempfile::tempdir().unwrap();
    let (gone, interrupted) = (dir.path().join("1"), dir.path().join("2"));
    std::fs::create_dir(&interrupted).unwrap();
    std::fs::write(interrupted.join("0001.jpg.part"), b"half").unwrap();
    library::mark_downloaded(&lib, "a", &chapters[0].id, &gone).unwrap();
    library::mark_downloaded(&lib, "a", &chapters[1].id, &interrupted).unwrap();

    let queued = queue::enqueue(&lib, "a", chapters, false).unwrap();
    assert_eq!((queued.jobs.len(), queued.on_disk.len()), (2, 0));
}
