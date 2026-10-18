//! SiteClient transport edge cases: bodies cut mid-stream and off-site redirects.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use mangas_providers::http::{ClientConfig, RetryPolicy, SiteClient};
use mangas_providers::{mangadot, Session};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

const FAST: RetryPolicy = RetryPolicy {
    max_retries: 2,
    base_delay: Duration::from_millis(1),
    max_delay: Duration::from_millis(50),
};

fn client(base: &str, session: Option<Session>) -> SiteClient {
    let config = ClientConfig {
        retry: FAST,
        requests_per_second: 1000,
        ..ClientConfig::new(mangadot::INFO, base)
    };
    SiteClient::new(config, session).unwrap()
}

/// Serves `replies` in order, one per connection. With `hold` the connections stay open until
/// the last reply is sent, so a body promised by `Content-Length` never arrives.
fn raw_server(replies: Vec<&'static str>, hold: bool) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let mut open = Vec::new();
        for reply in replies {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            stream.write_all(reply.as_bytes()).unwrap();
            if hold {
                open.push(stream);
            }
        }
    });
    format!("http://{addr}")
}

const PONG: &str = "HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\npong";

#[tokio::test]
async fn a_get_cut_off_mid_body_is_retried() {
    let short = "HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nshort";
    let base = raw_server(vec![short, PONG], false);
    let body = client(&base, None).get("/ping", &[]).await.unwrap();
    assert_eq!(body, "pong");
}

#[tokio::test]
async fn a_retried_response_body_is_not_awaited() {
    let stalled =
        "HTTP/1.1 503 Service Unavailable\r\nRetry-After: 0\r\nContent-Length: 100\r\n\r\n";
    let base = raw_server(vec![stalled, PONG], true);
    let request = client(&base, None);
    let body = tokio::time::timeout(Duration::from_secs(5), request.get("/ping", &[]))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(body, "pong");
}

#[tokio::test]
async fn off_site_redirects_drop_the_session_cookie() {
    let site = MockServer::start().await;
    let elsewhere = MockServer::start().await;
    Mock::given(path("/api"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/landing", elsewhere.uri())),
        )
        .mount(&site)
        .await;
    Mock::given(path("/landing"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&elsewhere)
        .await;
    let session = Session {
        cookies: "cf_clearance=secret".into(),
        user_agent: "UA".into(),
        saved_at: None,
    };
    let body = client(&site.uri(), Some(session))
        .get("/api", &[])
        .await
        .unwrap();
    assert_eq!(body, "ok");
    let sent = &site.received_requests().await.unwrap()[0];
    assert!(sent.headers.contains_key("cookie"));
    let landed = &elsewhere.received_requests().await.unwrap()[0];
    assert!(!landed.headers.contains_key("cookie"));
}

#[tokio::test]
async fn a_slow_call_can_take_longer_than_the_client_timeout() {
    let server = MockServer::start().await;
    Mock::given(path("/list"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("671 rows")
                .set_delay(Duration::from_millis(600)),
        )
        .mount(&server)
        .await;
    let config = ClientConfig {
        retry: RetryPolicy::NONE,
        requests_per_second: 1000,
        timeout: Duration::from_millis(200),
        ..ClientConfig::new(mangadot::INFO, &server.uri())
    };
    let client = SiteClient::new(config, None).unwrap();
    assert!(client.get("/list", &[]).await.is_err(), "default timeout");
    let long = client.get_within("/list", &[], Duration::from_secs(5));
    assert_eq!(long.await.unwrap(), "671 rows");
}
