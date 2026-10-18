//! The MangaDex edge's HTML `400` page: retried for JSON APIs, a real error everywhere else.

use std::time::Duration;

use mangas_providers::http::{ClientConfig, RetryPolicy, SiteClient};
use mangas_providers::{mangadex, mangadot};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FAST: RetryPolicy = RetryPolicy {
    max_retries: 2,
    base_delay: Duration::from_millis(1),
    max_delay: Duration::from_millis(50),
};

fn client(server: &MockServer, json_api: bool) -> SiteClient {
    let info = if json_api {
        mangadex::INFO
    } else {
        mangadot::INFO
    };
    let config = ClientConfig {
        retry: FAST,
        requests_per_second: 1000,
        json_api,
        ..ClientConfig::new(info, &server.uri())
    };
    SiteClient::new(config, None).unwrap()
}

fn edge_page() -> ResponseTemplate {
    let body = "<!doctype html><html><body>400: Bad Request</body></html>";
    ResponseTemplate::new(400).set_body_raw(body, "text/html; charset=utf-8")
}

async fn respond(server: &MockServer, response: ResponseTemplate, times: Option<u64>) {
    let mock = Mock::given(path("/feed")).respond_with(response);
    let mock = match times {
        Some(n) => mock.up_to_n_times(n).expect(n),
        None => mock,
    };
    mock.mount(server).await;
}

#[tokio::test]
async fn edge_error_page_is_retried_on_a_json_api() {
    let server = MockServer::start().await;
    respond(&server, edge_page(), Some(1)).await;
    respond(
        &server,
        ResponseTemplate::new(200).set_body_string("{}"),
        None,
    )
    .await;
    assert_eq!(client(&server, true).get("/feed", &[]).await.unwrap(), "{}");
}

#[tokio::test]
async fn edge_error_page_gives_up_after_the_retry_budget() {
    let server = MockServer::start().await;
    respond(&server, edge_page(), Some(3)).await;
    let err = client(&server, true)
        .get_raw("/feed", &[])
        .await
        .unwrap_err();
    let message = err.to_string();
    assert!(message.contains("HTTP 400 for /feed"), "{message}");
    assert!(message.contains("edge error page"), "{message}");
    assert!(message.contains("after 3 attempts"), "{message}");
}

#[tokio::test]
async fn json_400_is_a_request_error_and_not_retried() {
    let server = MockServer::start().await;
    let error = json!({"result": "error", "errors": [{"title": "Bad offset"}]});
    let response = ResponseTemplate::new(400).set_body_json(error);
    respond(&server, response, Some(1)).await;
    let reply = client(&server, true).get_raw("/feed", &[]).await.unwrap();
    assert_eq!(reply.status, 400);
    assert!(reply.body.contains("Bad offset"));
}

#[tokio::test]
async fn html_sites_keep_their_400s() {
    let server = MockServer::start().await;
    respond(&server, edge_page(), Some(1)).await;
    let reply = client(&server, false).get_raw("/feed", &[]).await.unwrap();
    assert_eq!(reply.status, 400);
}

#[tokio::test]
async fn posts_are_not_retried_on_an_edge_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/feed"))
        .respond_with(edge_page())
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server, true);
    let sent = client
        .post_json::<_, serde_json::Value>("/feed", &json!({}))
        .await;
    assert!(sent.is_err());
}
