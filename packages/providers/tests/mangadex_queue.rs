//! A chapter MangaDex answers with 404 fails its download job at once, as not found.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, Result};
use mangas_core::download::queue::{
    self, run_job, Job, JobRunner, JobState, Queue, QueueConfig, Scope,
};
use mangas_core::download::OnEvent;
use mangas_core::library::{self, Library};
use mangas_providers::http::PageClient;
use mangas_providers::mangadex::MangaDex;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The apps' runner, on a MangaDex pointed at the mock server.
struct MockedMangaDex {
    source: MangaDex,
    root: PathBuf,
}

#[async_trait]
impl JobRunner for MockedMangaDex {
    async fn run(&self, lib: &Library, job: &Job, on_event: OnEvent<'_>) -> Result<()> {
        let pages = PageClient::for_source(&self.source)?;
        run_job(lib, &self.source, &pages, job, &self.root, on_event).await?;
        Ok(())
    }
}

const NO_ROUTE: &str = r#"{"result":"error","errors":[{"id":"x","status":404,"title":"no_route_found_http_exception","detail":"No route found for \"GET /at-home/server/ghost-404\"","context":null}]}"#;

#[tokio::test(flavor = "multi_thread")]
async fn a_chapter_mangadex_answers_404_fails_at_once_as_not_found() {
    let server = MockServer::start().await;
    Mock::given(path("/at-home/server/ghost-404"))
        .respond_with(ResponseTemplate::new(404).set_body_string(NO_ROUTE))
        .expect(1)
        .mount(&server)
        .await;
    let lib = Arc::new(library::open_in_memory().unwrap());
    let manga = Manga {
        id: MangaId("m".into()),
        source: "mangadex".into(),
        title: "Ghost".into(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    let chapter = Chapter {
        id: ChapterId("ghost-404".into()),
        manga_id: manga.id.clone(),
        language: "en".into(),
        number: "1".into(),
        ..Default::default()
    };
    library::upsert_chapters(
        &lib,
        "mangadex",
        &manga.id,
        "en",
        std::slice::from_ref(&chapter),
    )
    .unwrap();
    queue::enqueue(&lib, "mangadex", vec![chapter], false).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let runner = MockedMangaDex {
        source: MangaDex::with_base(&server.uri()).unwrap(),
        root: dir.path().to_path_buf(),
    };
    let worker = Queue::new(lib.clone(), Arc::new(runner), QueueConfig::default());
    worker.run(Scope::All, true, &|_| {}).await.unwrap();
    let job = &queue::list(&lib).unwrap()[0];
    assert_eq!((job.state, job.attempts), (JobState::Failed, 1));
    let error = job.error.as_deref().unwrap();
    assert!(
        error.starts_with("not found: mangadex: No route found"),
        "{error}"
    );
    assert!(
        error.contains("(HTTP 404 for /at-home/server/ghost-404)"),
        "{error}"
    );
}
