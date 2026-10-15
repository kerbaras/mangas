use std::collections::HashSet;
use std::sync::Mutex;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Error, Manga, MangaId};
use mangas_core::download::{
    download_chapter_with, download_pages, page_file_name, plan, Event, PageFetcher,
};
use mangas_core::sources::{PageUrls, Source};

/// Serves page bodies from memory and records which URLs were asked for.
#[derive(Default)]
struct FakeFetcher {
    fetched: Mutex<Vec<String>>,
    fail_on: Option<String>,
}

#[async_trait]
impl PageFetcher for FakeFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, Error> {
        self.fetched.lock().unwrap().push(url.to_string());
        if self.fail_on.as_deref() == Some(url) {
            return Err(Error::Other(format!("boom {url}")));
        }
        Ok(format!("bytes of {url}").into_bytes())
    }
}

struct FakeSource(PageUrls);

#[async_trait]
impl Source for FakeSource {
    fn name(&self) -> &'static str {
        "fake"
    }
    async fn search(&self, _: &str, _: usize) -> Result<Vec<Manga>, Error> {
        Ok(vec![])
    }
    async fn manga(&self, id: &MangaId) -> Result<Manga, Error> {
        Err(Error::NotFound(id.0.clone()))
    }
    async fn chapters(&self, _: &MangaId, _: &str) -> Result<Vec<Chapter>, Error> {
        Ok(vec![])
    }
    async fn pages(&self, _: &ChapterId) -> Result<PageUrls, Error> {
        Ok(self.0.clone())
    }
}

fn urls(n: usize) -> PageUrls {
    PageUrls(
        (1..=n)
            .map(|i| {
                format!(
                    "https://cdn.example/data/hash/{i}-abc.{}",
                    if i % 2 == 0 { "jpg" } else { "png" }
                )
            })
            .collect(),
    )
}

fn chapter(number: &str) -> Chapter {
    Chapter {
        id: ChapterId(format!("ch-{number}")),
        manga_id: MangaId("m".into()),
        title: None,
        language: "en".into(),
        volume: None,
        number: number.into(),
        local_path: None,
    }
}

fn snapshot(events: &Mutex<Vec<Event>>) -> Vec<Event> {
    events.lock().unwrap().clone()
}

#[test]
fn page_names_are_zero_padded_with_source_extension() {
    assert_eq!(page_file_name(1, "https://x/1-abc.png"), "0001.png");
    assert_eq!(
        page_file_name(12, "https://x/12-abc.JPG?token=1"),
        "0012.jpg"
    );
    assert_eq!(page_file_name(3, "https://x/noext"), "0003.jpg");
}

#[test]
fn plan_skips_existing_files() {
    let existing: HashSet<String> = ["0001.png".to_string(), "0002.jpg".to_string()].into();
    let jobs = plan(&urls(4), &existing);
    let indexes: Vec<usize> = jobs.iter().map(|j| j.index).collect();
    assert_eq!(indexes, [3, 4]);
    assert_eq!(jobs[0].file_name, "0003.png");
}

#[tokio::test]
async fn download_resumes_and_reports_every_page() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("ch1");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("0002.jpg"), b"already here").unwrap();
    std::fs::write(dest.join("0003.png.part"), b"truncated").unwrap();

    let fetcher = FakeFetcher::default();
    let events = Mutex::new(Vec::new());
    let on_event = |e: Event| events.lock().unwrap().push(e);
    let total = download_pages(&fetcher, &ChapterId("c".into()), &urls(5), &dest, &on_event)
        .await
        .unwrap();
    assert_eq!(total, 5);

    let fetched = fetcher.fetched.lock().unwrap().clone();
    assert_eq!(fetched.len(), 4, "page 2 was skipped: {fetched:?}");
    assert!(!fetched.iter().any(|u| u.contains("/2-abc")));
    assert_eq!(
        std::fs::read(dest.join("0002.jpg")).unwrap(),
        b"already here"
    );
    assert!(std::fs::read(dest.join("0003.png"))
        .unwrap()
        .starts_with(b"bytes of"));
    assert!(!dest.join("0003.png.part").exists());

    let mut pages: Vec<usize> = snapshot(&events)
        .into_iter()
        .filter_map(|e| match e {
            Event::PageDone { page, total: 5, .. } => Some(page),
            _ => None,
        })
        .collect();
    pages.sort();
    assert_eq!(pages, [1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn second_run_fetches_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("ch");
    let on_event = |_: Event| {};
    let first = FakeFetcher::default();
    download_pages(&first, &ChapterId("c".into()), &urls(3), &dest, &on_event)
        .await
        .unwrap();
    let second = FakeFetcher::default();
    download_pages(&second, &ChapterId("c".into()), &urls(3), &dest, &on_event)
        .await
        .unwrap();
    assert!(second.fetched.lock().unwrap().is_empty());
}

#[tokio::test]
async fn failed_page_is_an_error_and_leaves_no_partial_file() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("ch");
    let fetcher = FakeFetcher {
        fail_on: Some(urls(3).0[1].clone()),
        ..Default::default()
    };
    let on_event = |_: Event| {};
    let err = download_pages(&fetcher, &ChapterId("c".into()), &urls(3), &dest, &on_event)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("boom"));
    assert!(!dest.join("0002.jpg").exists());
}

#[tokio::test]
async fn download_chapter_emits_chapter_done() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("ch");
    let source = FakeSource(urls(2));
    let fetcher = FakeFetcher::default();
    let events = Mutex::new(Vec::new());
    let on_event = |e: Event| events.lock().unwrap().push(e);
    let pages = download_chapter_with(&source, &fetcher, &chapter("1"), &dest, &on_event)
        .await
        .unwrap();
    assert_eq!(pages, 2);
    let last = snapshot(&events).pop().unwrap();
    assert_eq!(
        last,
        Event::ChapterDone {
            chapter: ChapterId("ch-1".into()),
            path: dest.to_string_lossy().into_owned(),
            pages: 2
        }
    );
}

#[tokio::test]
async fn chapter_without_pages_is_a_source_error() {
    let dir = tempfile::tempdir().unwrap();
    let source = FakeSource(PageUrls(vec![]));
    let fetcher = FakeFetcher::default();
    let on_event = |_: Event| {};
    let err = download_chapter_with(&source, &fetcher, &chapter("1"), dir.path(), &on_event)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Source { .. }), "{err}");
}
