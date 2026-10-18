mod queue_support;

use mangas_core::download::queue::{
    self, gate_event, Action, GateStatus, JobState, QueueEvent, Target, UNLISTED,
};
use mangas_core::library;
use queue_support::{follow, states};

#[test]
fn dismiss_removes_a_failed_job_and_refuses_one_on_its_way() {
    let lib = library::open_in_memory().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    queue::control(&lib, Target::Job(1), Action::Cancel).unwrap();

    let refused = queue::dismiss(&lib, 2).unwrap_err().to_string();
    assert!(refused.contains("download 2 is queued"), "{refused}");
    queue::dismiss(&lib, 1).unwrap();
    assert_eq!(states(&queue::list(&lib).unwrap()), [JobState::Queued]);
    assert!(queue::dismiss(&lib, 9)
        .unwrap_err()
        .to_string()
        .contains("not found"));
}

/// Fails the queued jobs of chapters the source no longer lists, by syncing an empty list.
fn drop_from_source(lib: &library::Library) {
    let manga = mangas_core::domain::MangaId("m".into());
    library::upsert_chapters(lib, "a", &manga, "en", &[]).unwrap();
}

fn chapter_count(lib: &library::Library) -> usize {
    let manga = mangas_core::domain::MangaId("m".into());
    library::list_chapters(lib, "a", &manga, None)
        .unwrap()
        .len()
}

#[test]
fn dismissing_an_unlisted_job_drops_the_chapter_kept_for_it() {
    let lib = library::open_in_memory().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    drop_from_source(&lib);
    let jobs = queue::list(&lib).unwrap();
    assert!(jobs.iter().all(|j| j.error.as_deref() == Some(UNLISTED)));
    assert_eq!(chapter_count(&lib), 2, "kept while their jobs are listed");

    queue::dismiss(&lib, 1).unwrap();
    assert_eq!(chapter_count(&lib), 1);
    assert_eq!(queue::list(&lib).unwrap().len(), 1);
}

#[test]
fn clearing_a_cancelled_unlisted_job_drops_its_chapter_too() {
    let lib = library::open_in_memory().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    drop_from_source(&lib);
    queue::control(&lib, Target::Job(2), Action::Cancel).unwrap();

    assert_eq!(queue::clear_finished(&lib).unwrap(), 1);
    assert_eq!(chapter_count(&lib), 1);
    assert_eq!(queue::list(&lib).unwrap()[0].id, 1);
}

#[test]
fn a_chapter_dropped_again_after_relisting_goes_with_its_job() {
    let lib = library::open_in_memory().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    drop_from_source(&lib);
    follow(&lib, "a", 2);
    drop_from_source(&lib);
    let jobs = queue::list(&lib).unwrap();
    assert!(jobs.iter().all(|j| j.error.as_deref() == Some(UNLISTED)));
    assert_eq!(chapter_count(&lib), 2, "kept while their jobs are listed");

    queue::dismiss(&lib, 1).unwrap();
    queue::dismiss(&lib, 2).unwrap();
    assert_eq!(chapter_count(&lib), 0);
}

#[test]
fn a_chapter_listed_again_survives_dismissing_its_job() {
    let lib = library::open_in_memory().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    drop_from_source(&lib);
    follow(&lib, "a", 2);
    let jobs = queue::list(&lib).unwrap();
    assert!(
        jobs.iter()
            .all(|j| j.state == JobState::Failed && j.error.is_none()),
        "{jobs:?}"
    );

    queue::dismiss(&lib, 1).unwrap();
    queue::dismiss(&lib, 2).unwrap();
    assert!(queue::list(&lib).unwrap().is_empty());
    assert_eq!(chapter_count(&lib), 2);
}

#[test]
fn clearing_keeps_chapters_the_source_still_lists() {
    let lib = library::open_in_memory().unwrap();
    queue::enqueue(&lib, "a", follow(&lib, "a", 2), false).unwrap();
    queue::control(&lib, Target::All, Action::Cancel).unwrap();
    assert_eq!(queue::clear_finished(&lib).unwrap(), 2);
    assert_eq!(chapter_count(&lib), 2);
}

#[test]
fn gate_status_reports_pause_all_and_a_move_hold() {
    let lib = library::open_in_memory().unwrap();
    assert_eq!(queue::gate_status(&lib).unwrap(), GateStatus::default());
    queue::control(&lib, Target::All, Action::Pause).unwrap();
    library::set_queue_held_until(&lib, queue::now_ms() + 60_000).unwrap();
    let status = queue::gate_status(&lib).unwrap();
    assert!(status.paused && status.held);
}

#[test]
fn a_gate_event_is_sent_first_and_then_only_on_change() {
    let open = GateStatus::default();
    let paused = GateStatus {
        paused: true,
        held: false,
    };
    assert!(gate_event(None, open).is_some());
    assert!(gate_event(Some(open), open).is_none());
    let event = gate_event(Some(open), paused).unwrap();
    assert_eq!(
        serde_json::to_value(&event).unwrap(),
        serde_json::json!({ "kind": "gate", "paused": true, "held": false })
    );
    assert!(matches!(event, QueueEvent::Gate { paused: true, .. }));
}
