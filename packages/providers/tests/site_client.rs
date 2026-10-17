//! SiteClient against a local mock server: retries, Retry-After, challenges, JSON, status.

use std::time::Duration;

use mangas_core::domain::Error;
use mangas_providers::http::{ClientConfig, RetryPolicy, SiteClient};
use mangas_providers::{mangadex, mangadot, ProviderStatus, Session};
use serde_json::{json, Value};
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FAST: RetryPolicy = RetryPolicy {
    max_retries: 2,
    base_delay: Duration::from_millis(1),
    max_delay: Duration::from_millis(50),
};

fn client(server: &MockServer) -> SiteClient {
    let config = ClientConfig {
        retry: FAST,
        requests_per_second: 1000,
        ..ClientConfig::new(mangadex::INFO, &server.uri())
    };
    SiteClient::new(config, None).unwrap()
}

/// Answers `status` `times` times on `route`, then falls through to later mocks.
async fn fail(server: &MockServer, route: &str, response: ResponseTemplate, times: u64) {
    Mock::given(path(route))
        .respond_with(response)
        .up_to_n_times(times)
        .expect(times)
        .mount(server)
        .await;
}

async fn ok(server: &MockServer, route: &str, body: &str) {
    Mock::given(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(server)
        .await;
}

#[tokio::test]
async fn server_errors_are_retried_with_backoff() {
    let server = MockServer::start().await;
    fail(&server, "/a", ResponseTemplate::new(503), 1).await;
    fail(&server, "/a", ResponseTemplate::new(502), 1).await;
    ok(&server, "/a", "done").await;
    assert_eq!(client(&server).get("/a", &[]).await.unwrap(), "done");
}

#[tokio::test]
async fn rate_limits_honour_retry_after() {
    let server = MockServer::start().await;
    let limited = ResponseTemplate::new(429).insert_header("retry-after", "0");
    fail(&server, "/a", limited, 1).await;
    ok(&server, "/a", "done").await;
    assert_eq!(client(&server).get("/a", &[]).await.unwrap(), "done");
}

#[tokio::test]
async fn retry_after_beyond_the_cap_is_not_waited_for() {
    let server = MockServer::start().await;
    let limited = ResponseTemplate::new(429).insert_header("retry-after", "3600");
    fail(&server, "/a", limited, 1).await;
    let err = client(&server).get("/a", &[]).await.unwrap_err();
    assert!(err.to_string().contains("HTTP 429"), "{err}");
}

#[tokio::test]
async fn retries_stop_after_max_retries() {
    let server = MockServer::start().await;
    fail(&server, "/a", ResponseTemplate::new(500), 3).await;
    let err = client(&server).get("/a", &[]).await.unwrap_err();
    assert!(err.to_string().contains("HTTP 500"), "{err}");
}

#[tokio::test]
async fn client_errors_and_challenges_are_not_retried() {
    let server = MockServer::start().await;
    fail(&server, "/missing", ResponseTemplate::new(404), 1).await;
    let challenge = ResponseTemplate::new(503).insert_header("cf-mitigated", "challenge");
    fail(&server, "/walled", challenge, 1).await;
    let client = client(&server);
    assert!(client.get("/missing", &[]).await.is_err());
    let err = client.get("/walled", &[]).await.unwrap_err();
    assert!(
        matches!(err, Error::Challenge { ref name, ref site } if name == "mangadex" && site == "https://mangadex.org"),
        "{err}"
    );
}

#[tokio::test]
async fn json_helpers_decode_and_report_the_path() {
    let server = MockServer::start().await;
    ok(&server, "/item", r#"{"id":7}"#).await;
    ok(&server, "/broken", "<html>").await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .and(body_json(json!({"q": "solo"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"hits": 1})))
        .mount(&server)
        .await;
    let client = client(&server);
    let item: Value = client.get_json("/item", &[]).await.unwrap();
    assert_eq!(item["id"], 7);
    let err = client.get_json::<Value>("/broken", &[]).await.unwrap_err();
    assert!(
        err.to_string().contains("/broken: unexpected JSON"),
        "{err}"
    );
    let hits: Value = client
        .post_json("/search", &json!({"q": "solo"}))
        .await
        .unwrap();
    assert_eq!(hits["hits"], 1);
}

#[tokio::test]
async fn post_is_not_retried_on_ambiguous_server_errors() {
    let server = MockServer::start().await;
    fail(&server, "/search", ResponseTemplate::new(500), 1).await;
    let result = client(&server)
        .post_json::<_, Value>("/search", &json!({}))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn status_probe_maps_responses() {
    use mangas_providers::Source;
    let server = MockServer::start().await;
    ok(&server, "/ping", "pong").await;
    let source = mangadex::MangaDex::with_base(&server.uri()).unwrap();
    assert_eq!(source.status().await, ProviderStatus::Ready);

    let down = MockServer::start().await;
    fail(&down, "/ping", ResponseTemplate::new(500), 1).await;
    let source = mangadex::MangaDex::with_base(&down.uri()).unwrap();
    let status = source.status().await;
    assert!(
        matches!(status, ProviderStatus::Unreachable { ref reason } if reason.contains("HTTP 500")),
        "{status:?}"
    );
}

#[tokio::test]
async fn session_sites_report_needs_session() {
    use mangas_providers::Source;
    let server = MockServer::start().await;
    let api = format!("{}/api", server.uri());
    let without = mangadot::MangaDot::with_base(&server.uri(), &api, None).unwrap();
    assert_eq!(without.status().await, ProviderStatus::NeedsSession);

    let challenge = ResponseTemplate::new(403).set_body_string("cdn-cgi/challenge-platform");
    fail(&server, "/api/manga", challenge, 1).await;
    let session = Session {
        cookies: "cf_clearance=old".into(),
        user_agent: "UA".into(),
        saved_at: None,
    };
    let stale = mangadot::MangaDot::with_base(&server.uri(), &api, Some(session.clone())).unwrap();
    assert_eq!(stale.status().await, ProviderStatus::NeedsSession);

    ok(&server, "/api/manga", r#"{"manga_list":[]}"#).await;
    let fresh = mangadot::MangaDot::with_base(&server.uri(), &api, Some(session)).unwrap();
    assert_eq!(fresh.status().await, ProviderStatus::Ready);
}
