//! `providers check` over the saved languages: the first language with chapters is walked.

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, MangaStatus, Result};
use mangas_providers::doctor::{check_with, first_language, Report, Step};
use mangas_providers::{Header, PageUrls, ProviderStatus, Source};

/// A provider that hosts chapters in one language only.
struct OnlyIn(&'static str, ProviderStatus);

#[async_trait]
impl Source for OnlyIn {
    fn name(&self) -> &'static str {
        "fake"
    }
    async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Manga>> {
        Ok(vec![Manga {
            id: MangaId("a".into()),
            source: "fake".into(),
            title: "Title a".into(),
            description: String::new(),
            cover_url: None,
            status: MangaStatus::Unknown,
        }])
    }
    async fn manga(&self, _id: &MangaId) -> Result<Manga> {
        Ok(self.search("", 1).await?.remove(0))
    }
    async fn chapters(&self, id: &MangaId, language: &str) -> Result<Vec<Chapter>> {
        Ok((language == self.0)
            .then(|| Chapter {
                id: ChapterId("a-1".into()),
                manga_id: id.clone(),
                title: None,
                language: language.into(),
                volume: None,
                number: "1".into(),
                local_path: None,
                group: None,
            })
            .into_iter()
            .collect())
    }
    async fn pages(&self, _chapter: &ChapterId) -> Result<PageUrls> {
        Ok(PageUrls(vec!["https://img.test/1.png".into()]))
    }
    fn page_headers(&self, _url: &str) -> Vec<Header> {
        Vec::new()
    }
    async fn status(&self) -> ProviderStatus {
        self.1.clone()
    }
}

async fn check(saved: &[&str], status: ProviderStatus) -> (Report, Vec<String>) {
    let languages: Vec<String> = saved.iter().map(|l| l.to_string()).collect();
    let walked = std::sync::Mutex::new(Vec::new());
    let report = first_language(&languages, |lang| {
        walked.lock().unwrap().push(lang.clone());
        let source = OnlyIn("en", status.clone());
        async move {
            let ok = |_: String, _: Vec<Header>| async { Ok(200) };
            Ok::<_, String>(check_with(&source, "q", &lang, ok).await)
        }
    })
    .await
    .unwrap();
    (report, walked.into_inner().unwrap())
}

fn outcome(report: &Report, step: Step) -> std::result::Result<String, String> {
    let found = report.steps.iter().find(|s| s.step == step).unwrap();
    found.outcome.clone()
}

#[tokio::test]
async fn falls_through_to_the_first_saved_language_with_chapters() {
    let (report, walked) = check(&["es-la", "en"], ProviderStatus::Ready).await;
    assert!(report.passed(), "{report:?}");
    assert_eq!(walked, ["es-la", "en"]);
    let chapters = outcome(&report, Step::Chapters).unwrap();
    assert!(chapters.starts_with("1 (en) in \"Title a\""), "{chapters}");
    assert!(
        chapters.contains("skipped es-la: no es-la chapters"),
        "{chapters}"
    );
}

#[tokio::test]
async fn stops_at_the_first_language_that_has_chapters() {
    let (report, walked) = check(&["en", "es-la"], ProviderStatus::Ready).await;
    assert!(report.passed());
    assert_eq!(walked, ["en"]);
    assert!(!outcome(&report, Step::Chapters)
        .unwrap()
        .contains("skipped"));
}

#[tokio::test]
async fn fails_only_when_no_language_has_chapters() {
    let (report, walked) = check(&["es-la", "fr"], ProviderStatus::Ready).await;
    assert!(!report.passed());
    assert_eq!(walked, ["es-la", "fr"]);
    let reason = outcome(&report, Step::Chapters).unwrap_err();
    assert!(
        reason.starts_with("no language had chapters (es-la: "),
        "{reason}"
    );
    assert!(reason.contains("; fr: no fr chapters"), "{reason}");
}

#[tokio::test]
async fn a_status_failure_is_not_retried_per_language() {
    let (report, walked) = check(&["es-la", "en"], ProviderStatus::NeedsSession).await;
    assert!(!report.passed());
    assert_eq!(walked, ["es-la"]);
}
