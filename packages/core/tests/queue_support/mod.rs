//! Shared fixtures for the queue tests: a library with chapters and a scriptable runner.
#![allow(dead_code)]

pub mod pages;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Error, Manga, MangaId, Result};
use mangas_core::download::queue::{Job, JobRunner, JobState, Limits, QueueConfig, RetryPolicy};
use mangas_core::download::{Event, OnEvent};
use mangas_core::library::{self, Library};

pub fn chapter(id: &str, number: &str) -> Chapter {
    Chapter {
        id: ChapterId(id.into()),
        manga_id: MangaId("m".into()),
        title: None,
        language: "en".into(),
        volume: None,
        number: number.into(),
        local_path: None,
        group: None,
        availability: Default::default(),
        exported_at: None,
    }
}

/// Follows manga `m` on `source` with chapters `<source>-1..=n`.
pub fn follow(lib: &Library, source: &str, n: usize) -> Vec<Chapter> {
    let manga = Manga {
        id: MangaId("m".into()),
        source: source.into(),
        title: "Kaiju".into(),
        ..Default::default()
    };
    library::upsert_manga(lib, &manga).unwrap();
    let chapters: Vec<Chapter> = (1..=n)
        .map(|i| chapter(&format!("{source}-{i}"), &i.to_string()))
        .collect();
    library::upsert_chapters(lib, source, &manga.id, "en", &chapters).unwrap();
    chapters
}

pub fn fast_config() -> QueueConfig {
    QueueConfig {
        limits: Limits {
            per_provider: 2,
            global: 3,
        },
        retry: RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 20,
            max_delay_ms: 50,
        },
        tick: Duration::from_millis(10),
        stale_after_ms: 15_000,
    }
}

/// Runs each job for `delay`, reporting 4 pages, failing the first `fail_first` attempts of
/// every job with `error`. Records the highest concurrency per source and overall.
#[derive(Default)]
pub struct FakeRunner {
    pub delay: Mutex<Duration>,
    pub fail_first: usize,
    pub error: Option<fn() -> Error>,
    pub attempts: Mutex<HashMap<i64, usize>>,
    pub running: Mutex<HashMap<String, usize>>,
    pub max_per_source: Mutex<HashMap<String, usize>>,
    pub max_total: AtomicUsize,
    pub aborted: AtomicBool,
    /// Runs that have read their delay and begun; a test waits on this (not on the job's
    /// `Running` row, written before the task is first polled) before it stops a run.
    pub started: AtomicUsize,
}

struct Leave<'a>(&'a FakeRunner, String, bool);

impl Drop for Leave<'_> {
    fn drop(&mut self) {
        *self.0.running.lock().unwrap().get_mut(&self.1).unwrap() -= 1;
        if !self.2 {
            self.0.aborted.store(true, Ordering::SeqCst);
        }
    }
}

#[async_trait]
impl JobRunner for FakeRunner {
    async fn run(&self, _: &Library, job: &Job, on_event: OnEvent<'_>) -> Result<()> {
        let mut guard = Leave(self, job.source.clone(), false);
        {
            let mut running = self.running.lock().unwrap();
            let now = running.entry(job.source.clone()).or_default();
            *now += 1;
            let mut max = self.max_per_source.lock().unwrap();
            let m = max.entry(job.source.clone()).or_default();
            *m = (*m).max(*now);
            let total: usize = running.values().sum();
            self.max_total.fetch_max(total, Ordering::SeqCst);
        }
        let attempt = {
            let mut a = self.attempts.lock().unwrap();
            let n = a.entry(job.id).or_default();
            *n += 1;
            *n
        };
        let delay = *self.delay.lock().unwrap();
        self.started.fetch_add(1, Ordering::SeqCst);
        for page in 1..=4 {
            tokio::time::sleep(delay / 4).await;
            on_event(Event::PageDone {
                chapter: job.chapter_id.clone(),
                page,
                total: 4,
            });
        }
        guard.2 = true;
        match self.error {
            Some(error) if attempt <= self.fail_first => Err(error()),
            _ => Ok(()),
        }
    }
}

pub fn states(jobs: &[Job]) -> Vec<JobState> {
    jobs.iter().map(|j| j.state).collect()
}

/// Waits until `fake` has begun `runs` runs: from then on an abort drops a live run, which
/// records `aborted`, and a changed delay no longer applies to it.
pub async fn started(fake: &FakeRunner, runs: usize) {
    eventually(|| fake.started.load(Ordering::SeqCst) >= runs).await;
}

/// Polls `check` every 10 ms for up to 5 s.
pub async fn eventually(mut check: impl FnMut() -> bool) {
    for _ in 0..500 {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("condition not reached in 5 s");
}
