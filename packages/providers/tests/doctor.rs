//! `providers check` walk on a scripted source: step order, candidate selection, failures and
//! the printed table.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Error, Manga, MangaId, MangaStatus, Result};
use mangas_providers::doctor::{check, check_with, lines, Report, Step};
use mangas_providers::{Header, PageUrls, ProviderStatus, Source};

struct Fake {
    status: ProviderStatus,
    results: Vec<&'static str>,
    chapters: HashMap<&'static str, usize>,
}

fn manga(id: &str) -> Manga {
    Manga {
        id: MangaId(id.into()),
        source: "fake".into(),
        title: format!("Title {id}"),
        description: String::new(),
        cover_url: None,
        status: MangaStatus::Unknown,
    }
}

#[async_trait]
impl Source for Fake {
    fn name(&self) -> &'static str {
        "fake"
    }
    async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Manga>> {
        Ok(self.results.iter().map(|id| manga(id)).collect())
    }
    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        Ok(manga(&id.0))
    }
    async fn chapters(&self, id: &MangaId, language: &str) -> Result<Vec<Chapter>> {
        let count = self.chapters.get(id.0.as_str()).copied().unwrap_or(0);
        Ok((1..=count)
            .map(|n| Chapter {
                id: ChapterId(format!("{}-{n}", id.0)),
                manga_id: id.clone(),
                title: None,
                language: language.into(),
                volume: None,
                number: n.to_string(),
                local_path: None,
                group: None,
            })
            .collect())
    }
    async fn pages(&self, chapter: &ChapterId) -> Result<PageUrls> {
        match chapter.0.as_str() {
            "empty-1" => Ok(PageUrls(vec![])),
            "local-1" => Ok(PageUrls(vec!["http://127.0.0.1:9/1.png".into()])),
            id => Ok(PageUrls(vec![format!("https://img.test/{id}/1.png")])),
        }
    }
    fn page_headers(&self, _url: &str) -> Vec<Header> {
        vec![("referer".into(), "https://fake.test/".into())]
    }
    async fn status(&self) -> ProviderStatus {
        self.status.clone()
    }
}

fn fake(results: Vec<&'static str>, chapters: &[(&'static str, usize)]) -> Fake {
    Fake {
        status: ProviderStatus::Ready,
        results,
        chapters: chapters.iter().copied().collect(),
    }
}

async fn run(source: &Fake, http_status: u16) -> (Report, Vec<(String, Vec<Header>)>) {
    let fetched = Arc::new(Mutex::new(Vec::new()));
    let log = fetched.clone();
    let report = check_with(source, "q", "en", move |url, headers| {
        let log = log.clone();
        async move {
            log.lock().map_err(|e| e.to_string())?.push((url, headers));
            Ok(http_status)
        }
    })
    .await;
    let fetched = fetched.lock().unwrap().clone();
    (report, fetched)
}

#[tokio::test]
async fn a_full_walk_passes_and_fetches_the_first_page_with_its_headers() {
    let (report, fetched) = run(&fake(vec!["a"], &[("a", 3)]), 200).await;
    assert!(report.passed(), "{report:?}");
    assert_eq!(report.steps.len(), 6);
    let referer = ("referer".to_string(), "https://fake.test/".to_string());
    assert_eq!(
        fetched,
        vec![("https://img.test/a-1/1.png".into(), vec![referer])]
    );
    let table = lines(&report);
    assert_eq!(table.len(), 7);
    assert!(table[3].starts_with("chapters  ok"), "{}", table[3]);
    assert!(table[3].ends_with("3 (en) in \"Title a\""), "{}", table[3]);
    assert!(table[6].starts_with("OK fake: 6 steps in "), "{}", table[6]);
}

#[tokio::test]
async fn results_without_chapters_are_skipped() {
    let (report, _) = run(&fake(vec!["none", "b"], &[("b", 1)]), 200).await;
    assert!(report.passed(), "{report:?}");
    let chapters = &report.steps[3];
    let expected = "1 (en) in \"Title b\"; skipped \"Title none\": no en chapters";
    assert_eq!(chapters.outcome, Ok(expected.into()));
    assert_eq!(report.steps[2].outcome, Ok("Title none (none)".into()));
}

#[tokio::test]
async fn every_skipped_result_is_named() {
    let (report, _) = run(&fake(vec!["x", "y", "c"], &[("c", 2)]), 200).await;
    let expected = "2 (en) in \"Title c\"; skipped \"Title x\", \"Title y\": no en chapters";
    assert_eq!(report.steps[3].outcome, Ok(expected.into()));
}

#[tokio::test]
async fn no_chapters_anywhere_fails_and_stops() {
    let (report, fetched) = run(&fake(vec!["x", "y"], &[]), 200).await;
    assert_eq!(report.failed_step(), Some(Step::Chapters));
    assert_eq!(report.steps.len(), 4);
    let reason = report.steps[3].outcome.clone().unwrap_err();
    assert_eq!(
        reason,
        "no en chapters in the first 2 results: \"Title x\", \"Title y\""
    );
    assert!(fetched.is_empty());
    let table = lines(&report);
    assert_eq!(table[4], "pages     -              not run");
    assert_eq!(table[6], "FAIL fake: chapters failed");
}

#[tokio::test]
async fn a_missing_session_fails_at_status() {
    let mut source = fake(vec!["a"], &[("a", 1)]);
    source.status = ProviderStatus::NeedsSession;
    let (report, _) = run(&source, 200).await;
    assert_eq!(report.steps.len(), 1);
    assert_eq!(report.failed_step(), Some(Step::Status));
    let reason = report.steps[0].outcome.clone().unwrap_err();
    assert!(reason.contains("browser session"), "{reason}");
}

#[tokio::test]
async fn empty_search_empty_pages_and_bad_image_status_fail() {
    let (report, _) = run(&fake(vec![], &[]), 200).await;
    assert_eq!(report.failed_step(), Some(Step::Search));
    let (report, _) = run(&fake(vec!["empty"], &[("empty", 1)]), 200).await;
    assert_eq!(report.failed_step(), Some(Step::Pages));
    let (report, _) = run(&fake(vec!["a"], &[("a", 1)]), 403).await;
    assert_eq!(report.failed_step(), Some(Step::Image));
    assert!(!report.passed());
    assert_eq!(lines(&report)[6], "FAIL fake: image failed");
}

#[tokio::test]
async fn source_errors_are_reported_not_raised() {
    struct Down;
    #[async_trait]
    impl Source for Down {
        fn name(&self) -> &'static str {
            "down"
        }
        async fn search(&self, _: &str, _: usize) -> Result<Vec<Manga>> {
            Err(Error::Challenge {
                name: "down".into(),
                site: "https://down.test".into(),
            })
        }
        async fn manga(&self, id: &MangaId) -> Result<Manga> {
            Ok(manga(&id.0))
        }
        async fn chapters(&self, _: &MangaId, _: &str) -> Result<Vec<Chapter>> {
            Ok(vec![])
        }
        async fn pages(&self, _: &ChapterId) -> Result<PageUrls> {
            Ok(PageUrls(vec![]))
        }
    }
    let report = check_with(&Down, "q", "en", |_, _| async { Ok(200) }).await;
    assert_eq!(report.failed_step(), Some(Step::Search));
    let reason = report.steps[1].outcome.clone().unwrap_err();
    assert!(reason.contains("needs a browser session"), "{reason}");
}

#[tokio::test]
async fn the_real_image_fetch_refuses_non_public_hosts() {
    let report = check(&fake(vec!["local"], &[("local", 1)]), "q", "en").await;
    assert_eq!(report.failed_step(), Some(Step::Image));
    let reason = report.steps[5].outcome.clone().unwrap_err();
    assert!(reason.starts_with("not a public http(s) url"), "{reason}");
}
