//! A 429 with `Retry-After` on one page holds back every page on the same host, not only the
//! request that got it.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use mangas_core::domain::{Chapter, ChapterId, Error, Manga, MangaId, Result};
use mangas_core::download::PageFetcher;
use mangas_providers::http::{PageClient, RetryPolicy};
use mangas_providers::{PageUrls, Source};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct Site;

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
}

#[tokio::test]
async fn retry_after_pauses_every_page_on_the_host() {
    // A fresh port: a pooled server could reuse another test's host bucket.
    let server = MockServer::builder().start().await;
    let limited = ResponseTemplate::new(429).insert_header("retry-after", "1");
    Mock::given(method("GET"))
        .and(path("/1.png"))
        .respond_with(limited)
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    let png = ResponseTemplate::new(200).set_body_bytes(b"png".to_vec());
    Mock::given(method("GET"))
        .respond_with(png)
        .mount(&server)
        .await;
    let policy = RetryPolicy {
        max_retries: 2,
        base_delay: Duration::from_millis(10),
        max_delay: Duration::from_secs(5),
    };
    let client = PageClient::for_source(&Site)
        .and_then(PageClient::allow_private_hosts)
        .unwrap()
        .with_retry(policy);
    let (first, second) = (
        format!("{}/1.png", server.uri()),
        format!("{}/2.png", server.uri()),
    );
    let other_page = async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        let start = Instant::now();
        let body = client.fetch(&second).await;
        (body, start.elapsed())
    };
    let (body1, (body2, waited)) = tokio::join!(client.fetch(&first), other_page);
    assert_eq!(body1.unwrap(), b"png");
    assert_eq!(body2.unwrap(), b"png");
    assert!(
        waited >= Duration::from_millis(500),
        "page 2 not paused: {waited:?}"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}
