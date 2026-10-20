//! Through a proxy the proxy resolves page hosts (#123): a host only the proxy knows downloads,
//! with no local lookup, while private IP literals and `localhost` are still refused up front.

mod proxy;

use mangas_core::domain::Error;
use mangas_core::download::PageFetcher;
use mangas_core::library::{Network, Settings};
use mangas_providers::http::{PageClient, RetryPolicy, PAGE_RETRY};
use proxy::{Proxy, ONLY_PROXY};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

const ONCE: RetryPolicy = RetryPolicy {
    max_retries: 0,
    ..PAGE_RETRY
};

/// The proxy by name (`localhost`), so its own lookup goes through the page resolver too.
fn via(proxy: &Proxy) -> Network {
    Network::Proxy(format!("http://localhost:{}", proxy.port))
}

async fn page_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(path("/p.jpg"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"page".to_vec()))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn a_host_only_the_proxy_resolves_downloads() {
    let (proxy, server) = (Proxy::start().await, page_server().await);
    let source = mangas_providers::source("mangadex", &Settings::default()).unwrap();
    let client = PageClient::via(source.as_ref(), &via(&proxy)).unwrap();
    let url = format!("http://{ONLY_PROXY}:{}/p.jpg", server.address().port());
    assert_eq!(client.fetch(&url).await.unwrap(), b"page");
    assert_eq!(proxy.seen(), [format!("GET {url} HTTP/1.1")]);
}

#[tokio::test]
async fn an_https_page_is_tunnelled_without_a_local_lookup() {
    let (proxy, server) = (Proxy::start().await, page_server().await);
    let source = mangas_providers::source("mangadex", &Settings::default()).unwrap();
    let client = PageClient::via(source.as_ref(), &via(&proxy))
        .unwrap()
        .with_retry(ONCE);
    let authority = format!("{ONLY_PROXY}:{}", server.address().port());
    // The page server speaks plain http, so the TLS handshake inside the tunnel fails.
    let err = client
        .fetch(&format!("https://{authority}/p.jpg"))
        .await
        .unwrap_err();
    assert!(!matches!(err, Error::BlockedAddress { .. }), "{err:?}");
    assert_eq!(proxy.seen(), [format!("CONNECT {authority} HTTP/1.1")]);
}

#[tokio::test]
async fn private_literals_are_still_refused_before_the_proxy() {
    let proxy = Proxy::start().await;
    let source = mangas_providers::source("mangadex", &Settings::default()).unwrap();
    let client = PageClient::via(source.as_ref(), &via(&proxy)).unwrap();
    for url in [
        "http://127.0.0.1:9/1.png",
        "https://10.0.0.5/x.jpg",
        "http://[::1]/x.jpg",
        "http://localhost:8080/x.jpg",
        "http://cdn.localhost/x.jpg",
    ] {
        let err = client.fetch(url).await.unwrap_err();
        assert!(
            matches!(err, Error::BlockedAddress { .. }),
            "{url}: {err:?}"
        );
    }
    assert!(proxy.seen().is_empty(), "{:?}", proxy.seen());
}

#[tokio::test]
async fn the_environment_proxy_resolves_too() {
    let (proxy, server) = (Proxy::start().await, page_server().await);
    std::env::set_var("HTTP_PROXY", format!("localhost:{}", proxy.port));
    let source = mangas_providers::source("mangadex", &Settings::default()).unwrap();
    let client = PageClient::via(source.as_ref(), &Network::System).unwrap();
    let url = format!("http://{ONLY_PROXY}:{}/p.jpg", server.address().port());
    assert_eq!(client.fetch(&url).await.unwrap(), b"page");
}
