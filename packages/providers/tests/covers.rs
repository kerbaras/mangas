//! CoverFetcher against a local mock server: page headers, retries, refusals.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Error, Manga, MangaId, Result};
use mangas_core::download::PageFetcher;
use mangas_providers::covers::CoverFetcher;
use mangas_providers::http::RetryPolicy;
use mangas_providers::{Header, PageUrls, Source};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FAST: RetryPolicy = RetryPolicy {
    max_retries: 2,
    base_delay: Duration::from_millis(1),
    max_delay: Duration::from_millis(50),
};
const REFERER: &str = "https://site.example/";

/// A source whose page policy sends the site referer everywhere, like a referer-gated CDN.
struct Gated;

#[async_trait]
impl Source for Gated {
    fn name(&self) -> &'static str {
        "gated"
    }
    async fn search(&self, _: &str, _: usize) -> Result<Vec<Manga>> {
        Ok(vec![])
    }
    async fn manga(&self, id: &MangaId) -> Result<Manga> {
        Err(Error::NotFound(id.0.clone()))
    }
    async fn chapters(&self, _: &MangaId, _: &str) -> Result<Vec<Chapter>> {
        Ok(vec![])
    }
    async fn pages(&self, _: &ChapterId) -> Result<PageUrls> {
        Ok(PageUrls(vec![]))
    }
    fn page_headers(&self, _: &str) -> Vec<Header> {
        vec![("referer".into(), REFERER.into())]
    }
}

fn fetcher() -> CoverFetcher {
    CoverFetcher::unguarded(Arc::new(Gated), FAST).unwrap()
}

/// The CDN answers 403 unless the request carries the site's referer.
async fn gated_cdn() -> MockServer {
    let server = MockServer::builder().start().await;
    Mock::given(method("GET"))
        .and(path("/cover.jpg"))
        .and(header("referer", REFERER))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"\xff\xd8\xffcover".to_vec()))
        .mount(&server)
        .await;
    Mock::given(path("/cover.jpg"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn sends_the_page_headers_of_the_source() {
    let server = gated_cdn().await;
    let body = fetcher()
        .fetch(&format!("{}/cover.jpg", server.uri()))
        .await
        .unwrap();
    assert_eq!(body, b"\xff\xd8\xffcover");
}

#[tokio::test]
async fn retries_rate_limited_requests() {
    let server = MockServer::builder().start().await;
    Mock::given(path("/c.png"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/c.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"png".to_vec()))
        .mount(&server)
        .await;
    let body = fetcher()
        .fetch(&format!("{}/c.png", server.uri()))
        .await
        .unwrap();
    assert_eq!(body, b"png");
}

#[tokio::test]
async fn http_errors_name_the_status() {
    let server = MockServer::builder().start().await;
    Mock::given(path("/gone.jpg"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let err = fetcher()
        .fetch(&format!("{}/gone.jpg", server.uri()))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("HTTP 404"), "{err}");
}

#[tokio::test]
async fn refuses_non_public_hosts_by_default() {
    let guarded = CoverFetcher::for_source(Arc::new(Gated)).unwrap();
    let err = guarded
        .fetch("http://127.0.0.1:1/cover.jpg")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("non-public"), "{err}");
}
