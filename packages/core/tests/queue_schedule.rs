use std::collections::HashMap;

use mangas_core::domain::ChapterId;
use mangas_core::download::queue::{
    diff, pick, plan_tick, Job, JobState, Limits, QueueConfig, QueueEvent, Scope, TickInput,
};

fn job(id: i64, source: &str, state: JobState) -> Job {
    Job {
        id,
        source: source.into(),
        chapter_id: ChapterId(format!("ch-{id}")),
        state,
        created_at: id,
        ..Job::default()
    }
}

fn running(id: i64, source: &str, owner: &str) -> Job {
    Job {
        owner: Some(owner.into()),
        attempts: 1,
        heartbeat_at: 1_000,
        ..job(id, source, JobState::Running)
    }
}

const NOW: i64 = 1_000;

#[test]
fn pick_respects_the_per_provider_and_global_caps_in_queue_order() {
    let mut jobs = vec![running(1, "a", "other")];
    jobs.extend((2..=8).map(|id| job(id, if id % 2 == 0 { "a" } else { "b" }, JobState::Queued)));
    let limits = Limits {
        per_provider: 2,
        global: 4,
    };
    // a already runs one, so it takes 2 and skips 4; b takes 3 and 5; the 4th overall stops.
    assert_eq!(pick(&jobs, limits, &Scope::All, NOW), [2, 3, 5]);
}

#[test]
fn two_releases_of_one_chapter_number_never_run_together() {
    let numbered = |id: i64, number: &str, state: JobState| Job {
        number: number.into(),
        ..job(id, "a", state)
    };
    let mut first = numbered(1, "5", JobState::Running);
    first.owner = Some("me".into());
    let jobs = vec![
        first,
        numbered(2, "5", JobState::Queued),
        numbered(3, "6", JobState::Queued),
        numbered(4, "6", JobState::Queued),
    ];
    let limits = Limits {
        per_provider: 4,
        global: 4,
    };
    assert_eq!(pick(&jobs, limits, &Scope::All, NOW), [3]);
}

#[test]
fn pick_skips_backoff_and_out_of_scope_jobs_without_blocking_the_rest() {
    let waiting = Job {
        not_before: NOW + 1,
        ..job(1, "a", JobState::Queued)
    };
    let jobs = vec![
        waiting,
        job(2, "a", JobState::Queued),
        job(3, "a", JobState::Queued),
    ];
    let scope = Scope::Jobs([1, 3].into());
    assert_eq!(pick(&jobs, Limits::default(), &scope, NOW), [3]);
}

#[test]
fn a_tick_finishes_results_heartbeats_and_claims() {
    let jobs = vec![
        running(1, "a", "me"),
        running(2, "a", "me"),
        job(3, "a", JobState::Queued),
        job(4, "a", JobState::Queued),
    ];
    let in_flight = HashMap::from([(2, Some((5, 20)))]);
    let finished = vec![(1, None)];
    let config = QueueConfig::default();
    let input = TickInput {
        owner: "me",
        now: 2_000,
        in_flight: &in_flight,
        finished: &finished,
        scope: &Scope::All,
        config: &config,
    };
    let plan = plan_tick(jobs, &input);
    let state = |id: i64| plan.jobs.iter().find(|j| j.id == id).unwrap().clone();
    assert_eq!(state(1).state, JobState::Done);
    assert_eq!((state(2).pages_done, state(2).heartbeat_at), (5, 2_000));
    assert_eq!(plan.claimed.iter().map(|j| j.id).collect::<Vec<_>>(), [3]);
    assert_eq!(state(3).attempts, 1);
    assert_eq!(
        state(4).state,
        JobState::Queued,
        "a: 2 running + 1 claimed would be 3"
    );
}

#[test]
fn diff_reports_state_changes_progress_and_removals_but_not_heartbeats() {
    let before = vec![
        running(1, "a", "me"),
        job(2, "a", JobState::Queued),
        job(3, "a", JobState::Done),
    ];
    let beat = Job {
        heartbeat_at: 9_999,
        ..before[0].clone()
    };
    assert!(diff(&before, &[beat, before[1].clone(), before[2].clone()]).is_empty());
    let progressed = Job {
        pages_done: 4,
        ..before[0].clone()
    };
    let started = running(2, "a", "me");
    let events = diff(&before, &[progressed, started]);
    assert!(matches!(&events[0], QueueEvent::Progress { job } if job.pages_done == 4));
    assert!(matches!(&events[1], QueueEvent::Started { job } if job.id == 2));
    assert_eq!(events[2], QueueEvent::Removed { id: 3 });
}

#[test]
fn events_serialise_with_a_kind_tag_and_the_job() {
    let json = serde_json::to_value(QueueEvent::Paused {
        job: job(7, "a", JobState::Paused),
    })
    .unwrap();
    assert_eq!(json["kind"], "paused");
    assert_eq!(json["job"]["state"], "paused");
    assert_eq!(json["job"]["chapter_id"], "ch-7");
}
