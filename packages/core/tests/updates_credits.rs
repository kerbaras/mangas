//! The update check re-fetches an uncredited title's record once, not on every check.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Error, Manga, MangaId};
use mangas_core::library::updates::{check, SourceFor, UpdatePolicy};
use mangas_core::library::{credits_checked, open_in_memory, upsert_manga, Library};
use mangas_core::sources::{PageUrls, Source};

fn manga(authors: &[&str]) -> Manga {
    Manga {
        id: MangaId("solo".into()),
        source: "fake".into(),
        title: "Untitled webcomic".into(),
        authors: authors.iter().map(|a| a.to_string()).collect(),
        ..Default::default()
    }
}

/// A source whose record names `authors`, counting record requests.
struct Counting {
    authors: Vec<&'static str>,
    records: Arc<AtomicUsize>,
}

#[async_trait]
impl Source for Counting {
    fn name(&self) -> &'static str {
        "fake"
    }
    async fn search(&self, _: &str, _: usize) -> Result<Vec<Manga>, Error> {
        Ok(vec![])
    }
    async fn manga(&self, _: &MangaId) -> Result<Manga, Error> {
        self.records.fetch_add(1, Ordering::SeqCst);
        Ok(manga(&self.authors))
    }
    async fn chapters(&self, id: &MangaId, _: &str) -> Result<Vec<Chapter>, Error> {
        Ok(vec![Chapter {
            id: ChapterId("c1".into()),
            manga_id: id.clone(),
            language: "en".into(),
            number: "1".into(),
            ..Default::default()
        }])
    }
    async fn pages(&self, _: &ChapterId) -> Result<PageUrls, Error> {
        Ok(PageUrls(vec![]))
    }
}

/// Runs `checks` update checks of a library holding `saved`; returns the record requests.
async fn record_requests(
    lib: &Library,
    saved: Manga,
    named: &[&'static str],
    checks: usize,
) -> usize {
    upsert_manga(lib, &saved).unwrap();
    let records = Arc::new(AtomicUsize::new(0));
    let source: Arc<dyn Source> = Arc::new(Counting {
        authors: named.to_vec(),
        records: records.clone(),
    });
    let resolve = move |_: &str| {
        Ok(SourceFor {
            source: source.clone(),
            languages: vec!["en".into()],
        })
    };
    for _ in 0..checks {
        check(lib, &resolve, &UpdatePolicy::default())
            .await
            .unwrap();
    }
    records.load(Ordering::SeqCst)
}

#[tokio::test]
async fn a_source_naming_nobody_is_asked_for_the_record_once_over_two_checks() {
    let lib = open_in_memory().unwrap();
    assert_eq!(record_requests(&lib, manga(&[]), &[], 2).await, 1);
    assert!(credits_checked(&lib, "fake", &MangaId("solo".into())).unwrap());
}

#[tokio::test]
async fn an_uncredited_title_gets_the_credits_the_source_now_names() {
    let lib = open_in_memory().unwrap();
    assert_eq!(record_requests(&lib, manga(&[]), &["SIU"], 2).await, 1);
    let saved = mangas_core::library::get_manga(&lib, "fake", &MangaId("solo".into())).unwrap();
    assert_eq!(saved.unwrap().authors, vec!["SIU".to_string()]);
}

#[tokio::test]
async fn a_credited_title_is_not_asked_for_its_record() {
    let lib = open_in_memory().unwrap();
    assert_eq!(record_requests(&lib, manga(&["SIU"]), &["SIU"], 2).await, 0);
}
