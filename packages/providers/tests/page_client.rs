//! Page downloads through `http::PageClient`: retries on transient statuses, a clear error and
//! no `ChapterDone` when a page cannot be fetched, page headers, ranged GET, declared CDN rates.

use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Error, Manga, MangaId, Result};
use mangas_core::download::{download_chapter_with, Event, PageFetcher};
use mangas_providers::http::{PageClient, RetryPolicy, DEFAULT_PAGE_RATE};
use mangas_providers::{Header, HostLimiters, PageUrls, Source};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FAST: RetryPolicy = RetryPolicy {
    max_retries: 2,
    base_delay: Duration::from_millis(10),
    max_delay: Duration::from_secs(2),
};

struct Site(Vec<String>);

#[async_trait]
impl Source for Site {
    fn name(&self) -> &'static str {
        "site"
    }
    async fn search(&self, _: &str, _: usize) -> Result<Vec<Manga>> {
        Ok(Vec::new())
    }
    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        Err(Error::NotFound(id.0.clone()))
    }
    async fn chapters(&self, _: &MangaId, _: &str) -> Result<Vec<Chapter>> {
        Ok(Vec::new())
    }
    async fn pages(&self, _: &ChapterId) -> Result<PageUrls> {
        Ok(PageUrls(self.0.clone()))
    }
    fn page_headers(&self, _url: &str) -> Vec<Header> {
        vec![("referer".into(), "https://site.example/".into())]
    }
}

fn chapter() -> Chapter {
    Chapter {
        id: ChapterId("c1".into()),
        manga_id: MangaId("m1".into()),
        title: None,
        language: "en".into(),
        volume: None,
        number: "1".into(),
        local_path: None,
        group: None,
    }
}

async fn page(server: &MockServer, route: &str, response: ResponseTemplate, times: Option<u64>) {
    let mock = Mock::given(method("GET"))
        .and(path(route))
        .and(header("referer", "https://site.example/"))
        .respond_with(response);
    match times {
        Some(n) => mock.up_to_n_times(n).with_priority(1).mount(server).await,
        None => mock.mount(server).await,
    }
}

fn png() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_bytes(b"png".to_vec())
}

/// Downloads a chapter of `routes` from `server`; returns the result and the events.
async fn download(
    server: &MockServer,
    routes: &[&str],
) -> (Result<usize>, Vec<Event>, tempfile::TempDir) {
    let urls = routes
        .iter()
        .map(|r| format!("{}{r}", server.uri()))
        .collect();
    let site = Site(urls);
    let client = client(&site).with_retry(FAST);
    let dir = tempfile::tempdir().unwrap();
    let events = Mutex::new(Vec::new());
    let on_event = |e: Event| events.lock().unwrap().push(e);
    let result = download_chapter_with(&site, &client, &chapter(), dir.path(), &on_event).await;
    (result, events.into_inner().unwrap(), dir)
}

fn client(site: &Site) -> PageClient<'_> {
    PageClient::for_source(site)
        .and_then(PageClient::allow_private_hosts)
        .unwrap()
}

fn chapter_done(events: &[Event]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, Event::ChapterDone { .. }))
}

#[tokio::test]
async fn transient_503_and_429_are_retried_and_the_chapter_completes() {
    let server = MockServer::start().await;
    page(&server, "/1.png", ResponseTemplate::new(503), Some(1)).await;
    page(&server, "/1.png", png(), None).await;
    let limited = ResponseTemplate::new(429).insert_header("retry-after", "0");
    page(&server, "/2.png", limited, Some(1)).await;
    page(&server, "/2.png", png(), None).await;
    let (result, events, dir) = download(&server, &["/1.png", "/2.png"]).await;
    assert_eq!(result.unwrap(), 2);
    assert!(chapter_done(&events));
    assert_eq!(std::fs::read(dir.path().join("0001.png")).unwrap(), b"png");
    assert_eq!(std::fs::read(dir.path().join("0002.png")).unwrap(), b"png");
    assert_eq!(server.received_requests().await.unwrap().len(), 4);
}

#[tokio::test]
async fn permanent_404_fails_the_chapter_with_the_page_count() {
    let server = MockServer::start().await;
    page(&server, "/1.png", png(), None).await;
    page(&server, "/2.png", ResponseTemplate::new(404), None).await;
    let (result, events, dir) = download(&server, &["/1.png", "/2.png"]).await;
    let err = result.unwrap_err().to_string();
    assert!(err.contains("1 of 2 pages failed"), "{err}");
    assert!(err.contains("page 2") && err.contains("HTTP 404"), "{err}");
    assert!(!chapter_done(&events), "{events:?}");
    assert!(dir.path().join("0001.png").exists(), "kept for a resume");
    assert!(!dir.path().join("0002.png").exists());
    // 404 is not retried.
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn retries_give_up_with_a_clear_error() {
    let server = MockServer::start().await;
    page(&server, "/1.png", ResponseTemplate::new(503), None).await;
    let (result, events, _dir) = download(&server, &["/1.png"]).await;
    let err = result.unwrap_err().to_string();
    assert!(err.contains("1 of 1 pages failed"), "{err}");
    assert!(err.contains("HTTP 503"), "{err}");
    assert!(!chapter_done(&events));
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn a_verification_page_is_a_challenge_error() {
    let server = MockServer::start().await;
    let page_challenge = ResponseTemplate::new(403).insert_header("cf-mitigated", "challenge");
    page(&server, "/1.png", page_challenge, None).await;
    let urls = vec![format!("{}/1.png", server.uri())];
    let site = Site(urls.clone());
    let client = client(&site);
    let err = client.fetch(&urls[0]).await.unwrap_err();
    assert!(matches!(err, Error::Challenge { .. }), "{err}");
}

#[tokio::test]
async fn first_byte_is_a_ranged_get_with_page_headers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/1.png"))
        .and(header("range", "bytes=0-0"))
        .and(header("referer", "https://site.example/"))
        .respond_with(ResponseTemplate::new(206))
        .mount(&server)
        .await;
    let site = Site(Vec::new());
    let client = client(&site);
    let url = format!("{}/1.png", server.uri());
    assert_eq!(client.first_byte(&url).await.unwrap(), 206);
}

#[tokio::test]
async fn non_public_page_urls_are_refused() {
    let site = Site(Vec::new());
    let client = PageClient::for_source(&site).unwrap();
    let err = client.fetch("http://127.0.0.1:1/1.png").await.unwrap_err();
    assert!(err.to_string().contains("non-public"), "{err}");
}

#[test]
fn image_hosts_get_their_provider_rate() {
    let pool = HostLimiters::default();
    pool.declare_image_hosts(&["cdn.example", "*"], 2);
    let declared = pool.page_bucket("https://img.cdn.example/a.png", DEFAULT_PAGE_RATE);
    assert_eq!(declared.limits(), (2, 2));
    let same = pool.page_bucket("https://img.cdn.example/b.png", DEFAULT_PAGE_RATE);
    assert!(std::sync::Arc::ptr_eq(&declared, &same));
    let other = pool.page_bucket("https://elsewhere.example/a.png", DEFAULT_PAGE_RATE);
    assert_eq!(other.limits(), (DEFAULT_PAGE_RATE, DEFAULT_PAGE_RATE));
    let api = pool.bucket("https://api.site.example", 3);
    let page = pool.page_bucket("https://api.site.example/page.png", DEFAULT_PAGE_RATE);
    assert!(
        std::sync::Arc::ptr_eq(&api, &page),
        "API bucket paces pages"
    );
}
