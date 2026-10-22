use mangas_core::domain::{ChapterId, Error};
use mangas_core::download::queue::{
    apply, backoff, claim, classify, enqueue_job, fail, finish, recover, Action, Failure, Job,
    JobState, RetryPolicy, UNLISTED,
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
fn actions_apply_only_where_they_make_sense() {
    use JobState::*;
    let cases = [
        (Queued, Action::Pause, Some(Paused)),
        (Running, Action::Pause, Some(Paused)),
        (Done, Action::Pause, None),
        (Paused, Action::Resume, Some(Queued)),
        (Queued, Action::Resume, None),
        (Paused, Action::Cancel, Some(Cancelled)),
        (Running, Action::Cancel, Some(Cancelled)),
        (Failed, Action::Cancel, Some(Cancelled)),
        (Done, Action::Cancel, None),
        (Failed, Action::Retry, Some(Queued)),
        (Cancelled, Action::Retry, Some(Queued)),
        (Running, Action::Retry, None),
    ];
    for (from, action, to) in cases {
        let next = apply(&running_or(from), action, NOW).map(|j| j.state);
        assert_eq!(next, to, "{from:?} + {action:?}");
    }
}

fn running_or(state: JobState) -> Job {
    match state {
        JobState::Running => running(1, "a", "me"),
        s => job(1, "a", s),
    }
}

#[test]
fn pausing_a_running_job_drops_its_owner_and_retry_resets_attempts() {
    let paused = apply(&running(1, "a", "me"), Action::Pause, NOW).unwrap();
    assert_eq!(paused.owner, None);
    let failed = Job {
        attempts: 3,
        error: Some("boom".into()),
        ..job(1, "a", JobState::Failed)
    };
    let retried = apply(&failed, Action::Retry, 5_000).unwrap();
    assert_eq!(
        (retried.attempts, retried.error, retried.created_at),
        (0, None, 5_000)
    );
}

#[test]
fn enqueue_keeps_an_active_job_and_requeues_a_finished_one() {
    let fresh = job(0, "a", JobState::Queued);
    assert!(enqueue_job(Some(&job(1, "a", JobState::Queued)), fresh.clone(), NOW).is_none());
    assert!(enqueue_job(Some(&running(1, "a", "me")), fresh.clone(), NOW).is_none());
    let resumed = enqueue_job(Some(&job(1, "a", JobState::Paused)), fresh.clone(), NOW).unwrap();
    assert_eq!(
        resumed.state,
        JobState::Queued,
        "asking again resumes a paused chapter"
    );
    let done = Job {
        pages_done: 20,
        ..job(1, "a", JobState::Done)
    };
    let again = enqueue_job(Some(&done), fresh.clone(), NOW).unwrap();
    assert_eq!(
        (again.state, again.pages_done, again.id),
        (JobState::Queued, 0, 1)
    );
    assert_eq!(enqueue_job(None, fresh, NOW).unwrap().created_at, NOW);
}

#[test]
fn a_retryable_failure_backs_off_until_attempts_run_out() {
    let policy = RetryPolicy::default();
    assert_eq!(
        [1, 2, 3, 9].map(|n| backoff(&policy, n)),
        [15_000, 60_000, 120_000, 120_000]
    );
    let network = Failure {
        message: "network: reset".into(),
        retryable: true,
    };
    let first = fail(&running(1, "a", "me"), "me", &network, &policy, NOW).unwrap();
    assert_eq!(first.state, JobState::Queued);
    assert_eq!(first.not_before, NOW + 15_000);
    assert_eq!(first.error.as_deref(), Some("network: reset"));
    let last = Job {
        attempts: 3,
        ..running(1, "a", "me")
    };
    assert_eq!(
        fail(&last, "me", &network, &policy, NOW).unwrap().state,
        JobState::Failed
    );
}

#[test]
fn challenges_and_disk_errors_fail_at_once() {
    let challenge = Error::Challenge {
        name: "MangaDot".into(),
        site: "https://mangadot.net".into(),
    };
    assert!(!classify(&challenge).retryable);
    let read_only = Error::Io {
        path: "/data".into(),
        source: std::io::Error::other("read-only"),
    };
    assert!(!classify(&read_only).retryable);
    assert!(
        classify(&Error::Other(
            "incomplete chapter: 2 of 9 pages failed".into()
        ))
        .retryable
    );
    let f = classify(&challenge);
    let failed = fail(
        &running(1, "a", "me"),
        "me",
        &f,
        &RetryPolicy::default(),
        NOW,
    )
    .unwrap();
    assert_eq!(failed.state, JobState::Failed);
}

#[test]
fn a_source_404_fails_at_once_as_not_found() {
    let gone = classify(&Error::Source {
        name: "mangadex".into(),
        message: "HTTP 404 for /at-home/server/x".into(),
    });
    assert!(!gone.retryable);
    assert!(gone.message.starts_with("not found:"), "{}", gone.message);
    let busy = classify(&Error::Source {
        name: "mangadex".into(),
        message: "HTTP 503 for /at-home/server/x".into(),
    });
    assert!(busy.retryable);
    let pages = classify(&Error::Other(
        "incomplete chapter: 2 of 9 pages failed (page 3: source x: HTTP 404 for /p3; page 4: io: reset)"
            .into(),
    ));
    assert!(!pages.retryable, "a page that is gone stays gone");
}

#[test]
fn retrying_an_unlisted_chapter_gets_one_final_attempt() {
    let unlisted = Job {
        attempts: 0,
        error: Some(UNLISTED.into()),
        ..job(1, "a", JobState::Failed)
    };
    let retried = apply(&unlisted, Action::Retry, NOW).unwrap();
    assert_eq!(retried.state, JobState::Queued);
    let running = claim(&retried, "me", NOW);
    let network = Failure {
        message: "source mangadex: chapter 7 has no pages".into(),
        retryable: true,
    };
    let failed = fail(&running, "me", &network, &RetryPolicy::default(), NOW).unwrap();
    assert_eq!((failed.state, failed.attempts), (JobState::Failed, 1));
    assert!(failed
        .error
        .unwrap()
        .starts_with("not found: chapter no longer listed"));
    // A retry that works clears the reason.
    assert_eq!(
        finish(&claim(&retried, "me", NOW), "me", NOW)
            .unwrap()
            .error,
        None
    );
}

#[test]
fn a_late_result_never_overrides_a_pause_or_another_owner() {
    let ok = Failure {
        message: "x".into(),
        retryable: true,
    };
    let paused = job(1, "a", JobState::Paused);
    assert!(fail(&paused, "me", &ok, &RetryPolicy::default(), NOW).is_none());
    assert!(fail(
        &running(1, "a", "them"),
        "me",
        &ok,
        &RetryPolicy::default(),
        NOW
    )
    .is_none());
}

#[test]
fn a_stale_running_job_is_recovered_without_counting_the_attempt() {
    let stale = running(1, "a", "gone");
    assert!(recover(&stale, 1_000 + 15_000, 15_000).is_none());
    let back = recover(&stale, 1_000 + 15_001, 15_000).unwrap();
    assert_eq!(
        (back.state, back.owner, back.attempts),
        (JobState::Queued, None, 0)
    );
}
