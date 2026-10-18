//! The per-host limiter: one bucket per host, shared by every client and call to that host.

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::future::join_all;
use mangas_providers::http::{ClientConfig, RetryPolicy, SiteClient};
use mangas_providers::{host_key, mangadex, HostLimiters};
use wiremock::matchers::path_regex;
use wiremock::{Mock, MockServer, ResponseTemplate};

const RATE: u32 = 4;

#[test]
fn host_key_is_host_and_port() {
    assert_eq!(host_key("https://api.mangadex.org"), "api.mangadex.org:443");
    assert_eq!(
        host_key("https://API.mangadex.org/v2/"),
        "api.mangadex.org:443"
    );
    assert_eq!(host_key("http://127.0.0.1:8080/api"), "127.0.0.1:8080");
    assert_eq!(host_key("not a url"), "not a url");
}

#[test]
fn one_bucket_per_host_with_the_strictest_rate() {
    let pool = HostLimiters::default();
    let site = pool.bucket("https://mangadot.net", 5);
    let api = pool.bucket("https://mangadot.net/api", 3);
    assert!(Arc::ptr_eq(&site, &api));
    assert_eq!(site.limits(), (3, 3));
    let other = pool.bucket("https://api.mangadex.org", 5);
    assert!(!Arc::ptr_eq(&site, &other));
    assert_eq!(other.limits(), (5, 5));
    assert_eq!(pool.bucket("https://mangadot.net", 10).limits(), (3, 3));
}

fn client(base: &str) -> SiteClient {
    let config = ClientConfig {
        requests_per_second: RATE,
        retry: RetryPolicy::NONE,
        ..ClientConfig::new(mangadex::INFO, base)
    };
    SiteClient::new(config, None).unwrap()
}

/// A fresh server: `MockServer::start` recycles pooled servers (and so their host:port) across
/// tests, which would hand a test a bucket another test already drained.
async fn server(expected: u64) -> MockServer {
    let server = MockServer::builder().start().await;
    Mock::given(path_regex("/ping$"))
        .respond_with(ResponseTemplate::new(200).set_body_string("pong"))
        .expect(expected)
        .mount(&server)
        .await;
    server
}

/// Sends `per_client` GETs from each client concurrently; returns how long they all took.
async fn burst(clients: &[SiteClient], per_client: usize) -> Duration {
    let start = Instant::now();
    let calls = clients
        .iter()
        .flat_map(|c| (0..per_client).map(move |_| c.get("/ping", &[])));
    for result in join_all(calls).await {
        assert_eq!(result.unwrap(), "pong");
    }
    start.elapsed()
}

#[tokio::test]
async fn clients_for_one_host_share_its_budget() {
    // 12 requests at 4/s with a burst of 4 need at least 2 s in total. Each client alone
    // would finish its 6 in 0.5 s, so this only passes when the two share one bucket.
    let server = server(12).await;
    let api = client(&format!("{}/api", server.uri()));
    let site = client(&server.uri());
    let took = burst(&[api, site], 6).await;
    assert!(took >= Duration::from_millis(1900), "took {took:?}");
}

#[tokio::test]
async fn sources_built_by_separate_calls_share_the_budget() {
    let server = server(8).await;
    let first = burst(&[client(&server.uri())], 4).await;
    assert!(
        first < Duration::from_millis(500),
        "first call took {first:?}"
    );
    // A new client for the same host starts from the bucket the first one drained.
    let second = burst(&[client(&server.uri())], 4).await;
    assert!(
        second >= Duration::from_millis(700),
        "second call took {second:?}"
    );
}

#[tokio::test]
async fn different_hosts_do_not_block_each_other() {
    let (a, b) = (server(4).await, server(4).await);
    let took = burst(&[client(&a.uri()), client(&b.uri())], 4).await;
    assert!(took < Duration::from_millis(900), "took {took:?}");
}
