//! Page and cover downloads follow redirects hop by hop (each hop on its own host's bucket and
//! headers), refuse names that resolve to private addresses, and cap cover bodies.

use std::net::TcpListener;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Error, Manga, MangaId, Result};
use mangas_core::download::PageFetcher;
use mangas_providers::covers::CoverFetcher;
use mangas_providers::http::{PageClient, RetryPolicy};
use mangas_providers::{Header, HostLimiters, PageUrls, Source};
use wiremock::matchers::{header, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Sends the site referer only to `site`, like a provider's [`mangas_providers::http::HostPolicy`],
/// and rewrites `/old` URLs to `/new` the way an https upgrade would.
struct Site(String);

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
        Ok(PageUrls(Vec::new()))
    }
    fn page_headers(&self, url: &str) -> Vec<Header> {
        match url.starts_with(&self.0) {
            true => vec![("referer".into(), "https://site.example/".into())],
            false => Vec::new(),
        }
    }
    fn page_url(&self, url: &str) -> String {
        url.replace("/old", "/new")
    }
}

const FAST: RetryPolicy = RetryPolicy {
    max_retries: 0,
    base_delay: Duration::from_millis(1),
    max_delay: Duration::from_millis(1),
};

fn client(site: &Site) -> PageClient<'_> {
    PageClient::for_source(site)
        .and_then(PageClient::allow_private_hosts)
        .unwrap()
        .with_retry(FAST)
}

async fn redirect(from: &MockServer, to: String) {
    Mock::given(path("/p.jpg"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", to))
        .mount(from)
        .await;
}

#[tokio::test]
async fn a_page_redirect_waits_for_the_destination_bucket_and_gets_its_headers() {
    let site = MockServer::start().await;
    let cdn = MockServer::start().await;
    redirect(&site, format!("{}/img.jpg", cdn.uri())).await;
    Mock::given(path("/img.jpg"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"img".to_vec()))
        .mount(&cdn)
        .await;
    HostLimiters::shared().bucket(&cdn.uri(), 2);
    let source = Site(site.uri());
    let pages = client(&source);
    let start = Instant::now();
    for _ in 0..4 {
        assert_eq!(
            pages.fetch(&format!("{}/p.jpg", site.uri())).await.unwrap(),
            b"img"
        );
    }
    assert!(
        start.elapsed() >= Duration::from_millis(900),
        "{:?}",
        start.elapsed()
    );
    let first = &site.received_requests().await.unwrap()[0];
    assert!(first.headers.contains_key("referer"));
    let landed = &cdn.received_requests().await.unwrap()[0];
    assert!(!landed.headers.contains_key("referer"));
}

#[tokio::test]
async fn the_source_page_url_is_what_gets_fetched() {
    let site = MockServer::start().await;
    Mock::given(path("/new.jpg"))
        .and(header("referer", "https://site.example/"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"new".to_vec()))
        .mount(&site)
        .await;
    let source = Site(site.uri());
    let body = client(&source)
        .fetch(&format!("{}/old.jpg", site.uri()))
        .await;
    assert_eq!(body.unwrap(), b"new");
}

#[tokio::test]
async fn a_name_resolving_to_loopback_is_refused_before_connecting() {
    // `ip6-localhost` is a plain name (passes the host text check) for ::1 on Linux.
    let Ok(listener) = TcpListener::bind("[::1]:0") else {
        return;
    };
    let resolves_to_loopback = tokio::net::lookup_host(("ip6-localhost", 0))
        .await
        .is_ok_and(|mut addrs| addrs.any(|a| a.ip().is_loopback()));
    if !resolves_to_loopback {
        return;
    }
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let source = Site(String::new());
    let guarded = PageClient::for_source(&source).unwrap().with_retry(FAST);
    let err = guarded
        .fetch(&format!("http://ip6-localhost:{port}/p.jpg"))
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("non-public address"), "{err:?}");
    assert!(listener.accept().is_err(), "the guarded client connected");
}

#[tokio::test]
async fn covers_over_the_size_limit_are_refused() {
    let cdn = MockServer::start().await;
    Mock::given(path("/huge.jpg"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8; 21 * 1024 * 1024]))
        .mount(&cdn)
        .await;
    let covers = CoverFetcher::unguarded(Arc::new(Site(cdn.uri())), FAST).unwrap();
    let err = covers
        .fetch(&format!("{}/huge.jpg", cdn.uri()))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("bytes"), "{err}");
}

#[tokio::test]
async fn covers_follow_redirects_by_hand() {
    let site = MockServer::start().await;
    let cdn = MockServer::start().await;
    redirect(&site, format!("{}/c.jpg", cdn.uri())).await;
    Mock::given(path("/c.jpg"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"cover".to_vec()))
        .mount(&cdn)
        .await;
    let covers = CoverFetcher::unguarded(Arc::new(Site(site.uri())), FAST).unwrap();
    let body = covers
        .fetch(&format!("{}/p.jpg", site.uri()))
        .await
        .unwrap();
    assert_eq!(body, b"cover");
}

#[tokio::test]
async fn caller_headers_stay_on_the_first_host() {
    let site = MockServer::start().await;
    let cdn = MockServer::start().await;
    redirect(&site, format!("{}/c.jpg", cdn.uri())).await;
    Mock::given(path("/c.jpg"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&cdn)
        .await;
    let source = Site(site.uri());
    let secret = ("authorization".to_string(), "secret".to_string());
    let fetched = client(&source)
        .get(&format!("{}/p.jpg", site.uri()), &[secret])
        .await
        .unwrap();
    assert_eq!(fetched.status, 200);
    let sent = &site.received_requests().await.unwrap()[0];
    assert!(sent.headers.contains_key("authorization"));
    let landed = &cdn.received_requests().await.unwrap()[0];
    assert!(!landed.headers.contains_key("authorization"));
}

#[tokio::test]
async fn a_large_error_page_reports_its_status_not_its_size() {
    let cdn = MockServer::start().await;
    Mock::given(path("/gone.jpg"))
        .respond_with(ResponseTemplate::new(404).set_body_bytes(vec![0u8; 21 * 1024 * 1024]))
        .mount(&cdn)
        .await;
    let covers = CoverFetcher::unguarded(Arc::new(Site(cdn.uri())), FAST).unwrap();
    let err = covers
        .fetch(&format!("{}/gone.jpg", cdn.uri()))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("404"), "{err}");
}
