//! SiteClient follows redirects by hand: each hop draws from its own host's bucket, and the
//! method/body follow RFC 9110 (301/302/303 turn a POST into a GET, 307/308 keep it).

use std::time::{Duration, Instant};

use mangas_providers::http::{ClientConfig, RetryPolicy, SiteClient};
use mangas_providers::mangadot;
use mangas_providers::HostLimiters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(base: &str) -> SiteClient {
    let config = ClientConfig {
        retry: RetryPolicy::NONE,
        requests_per_second: 1000,
        ..ClientConfig::new(mangadot::INFO, base)
    };
    SiteClient::new(config, None).unwrap()
}

async fn redirecting(status: u16, to: &MockServer) -> MockServer {
    let site = MockServer::start().await;
    Mock::given(path("/api"))
        .respond_with(
            ResponseTemplate::new(status).insert_header("location", format!("{}/land", to.uri())),
        )
        .mount(&site)
        .await;
    site
}

#[tokio::test]
async fn a_redirect_to_another_host_waits_for_that_hosts_bucket() {
    let slow = MockServer::start().await;
    Mock::given(path("/land"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&slow)
        .await;
    HostLimiters::shared().bucket(&slow.uri(), 2);
    let site = redirecting(302, &slow).await;
    let api = client(&site.uri());
    let start = Instant::now();
    for _ in 0..4 {
        assert_eq!(api.get("/api", &[]).await.unwrap(), "ok");
    }
    // A burst of 2 at 2/s: the 3rd and 4th hop to `slow` wait ~0.5 s each.
    assert!(
        start.elapsed() >= Duration::from_millis(900),
        "{:?}",
        start.elapsed()
    );
    assert_eq!(slow.received_requests().await.unwrap().len(), 4);
}

async fn post_through(status: u16) -> wiremock::Request {
    let landing = MockServer::start().await;
    Mock::given(path("/land"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&landing)
        .await;
    let site = redirecting(status, &landing).await;
    let reply: serde_json::Value = client(&site.uri())
        .post_json("/api", &serde_json::json!({ "q": 1 }))
        .await
        .unwrap();
    assert_eq!(reply, serde_json::json!({}));
    landing.received_requests().await.unwrap().remove(0)
}

#[tokio::test]
async fn a_post_redirected_with_303_becomes_a_bodyless_get() {
    for status in [301, 302, 303] {
        let landed = post_through(status).await;
        assert_eq!(landed.method.as_str(), "GET", "{status}");
        assert!(landed.body.is_empty(), "{status}");
    }
}

#[tokio::test]
async fn a_post_redirected_with_307_or_308_keeps_its_method_and_body() {
    for status in [307, 308] {
        let landed = post_through(status).await;
        assert_eq!(landed.method.as_str(), "POST", "{status}");
        assert_eq!(landed.body, br#"{"q":1}"#, "{status}");
    }
}

#[tokio::test]
async fn a_redirect_loop_gives_up() {
    let site = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/api"))
        .mount(&site)
        .await;
    let err = client(&site.uri()).get("/api", &[]).await.unwrap_err();
    assert!(err.to_string().contains("redirects"), "{err}");
}

async fn slow_redirect(to: String) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", to)
                .set_delay(Duration::from_millis(250)),
        )
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn a_redirect_chain_shares_one_timeout() {
    let land = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&land)
        .await;
    let second = slow_redirect(format!("{}/land", land.uri())).await;
    let first = slow_redirect(format!("{}/next", second.uri())).await;
    // Each hop alone fits in the timeout; the two together do not.
    let config = ClientConfig {
        retry: RetryPolicy::NONE,
        requests_per_second: 1000,
        timeout: Duration::from_millis(400),
        ..ClientConfig::new(mangadot::INFO, &first.uri())
    };
    let client = SiteClient::new(config, None).unwrap();
    assert!(client.get("/api", &[]).await.is_err());
    assert!(land.received_requests().await.unwrap().is_empty());
}
