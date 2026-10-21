//! A page host that can't be looked up, directly or by the proxy, fails after one retry with the
//! host and the cause (#311); a site's own 5xx through the proxy keeps every retry.

mod proxy;

use std::time::{Duration, Instant};

use mangas_core::download::PageFetcher;
use mangas_core::library::{Network, Settings};
use mangas_providers::http::{PageClient, RetryPolicy};
use proxy::{Proxy, ONLY_PROXY};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Ten retries 300 ms apart: at least 3 s if every one ran.
const PATIENT: RetryPolicy = RetryPolicy {
    max_retries: 10,
    base_delay: Duration::from_millis(300),
    max_delay: Duration::from_millis(300),
};

fn via(proxy: &Proxy) -> Network {
    Network::Proxy(format!("http://localhost:{}", proxy.port))
}

fn client<'a>(source: &'a dyn mangas_providers::Source, network: &Network) -> PageClient<'a> {
    PageClient::via(source, network)
        .unwrap()
        .with_retry(PATIENT)
}

#[tokio::test]
async fn a_name_that_does_not_resolve_fails_after_one_retry_with_the_cause() {
    let source = mangas_providers::source("mangadex", &Settings::default()).unwrap();
    let client = client(source.as_ref(), &Network::Direct);
    let started = Instant::now();
    let err = client
        .fetch("https://nowhere.invalid/p.jpg")
        .await
        .unwrap_err()
        .to_string();
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    assert!(
        err.starts_with("network: couldn't resolve nowhere.invalid: "),
        "{err}"
    );
    assert!(
        err.len() > "network: couldn't resolve nowhere.invalid: ".len(),
        "{err}"
    );
}

#[tokio::test]
async fn a_tunnel_the_proxy_refuses_fails_after_one_retry() {
    let proxy = Proxy::start().await;
    let source = mangas_providers::source("mangadex", &Settings::default()).unwrap();
    let err = client(source.as_ref(), &via(&proxy))
        .fetch("https://unknown.test/p.jpg")
        .await
        .unwrap_err()
        .to_string();
    assert_eq!(proxy.seen().len(), 2, "{:?}", proxy.seen());
    let expected = format!(
        "network: the proxy http://localhost:{} couldn't reach unknown.test (it refused the tunnel)",
        proxy.port
    );
    assert!(err.starts_with(&expected), "{err}");
}

#[tokio::test]
async fn the_proxy_s_own_5xx_for_plain_http_fails_after_one_retry() {
    let proxy = Proxy::start().await;
    let source = mangas_providers::source("mangadex", &Settings::default()).unwrap();
    let err = client(source.as_ref(), &via(&proxy))
        .fetch("http://unknown.test/p.jpg")
        .await
        .unwrap_err()
        .to_string();
    assert_eq!(proxy.seen().len(), 2, "{:?}", proxy.seen());
    assert!(
        err.contains("couldn't reach unknown.test (HTTP 502)"),
        "{err}"
    );
}

#[tokio::test]
async fn the_site_s_own_5xx_through_the_proxy_keeps_its_retries() {
    let (proxy, server) = (Proxy::start().await, MockServer::start().await);
    Mock::given(path("/p.jpg"))
        .respond_with(ResponseTemplate::new(503).insert_header("via", "1.1 tinyproxy"))
        .mount(&server)
        .await;
    let source = mangas_providers::source("mangadex", &Settings::default()).unwrap();
    let retry = RetryPolicy {
        max_retries: 3,
        base_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(10),
    };
    let client = PageClient::via(source.as_ref(), &via(&proxy))
        .unwrap()
        .with_retry(retry);
    let url = format!("http://{ONLY_PROXY}:{}/p.jpg", server.address().port());
    let err = client.fetch(&url).await.unwrap_err().to_string();
    let tries = server.received_requests().await.unwrap_or_default().len();
    assert_eq!(tries, 4, "{err}");
    assert!(
        err.contains("HTTP 503") && err.contains("4 attempts"),
        "{err}"
    );
}
