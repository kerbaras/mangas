//! MangaDot against a mock edge that, like Cloudflare's site rule on mangadot.net, answers the
//! challenge page to any `/api` request without a `Referer` (#94).

mod contract;

use contract::manifest::challenge_page;
use mangas_core::domain::MangaId;
use mangas_providers::mangadot::MangaDot;
use mangas_providers::{ProviderStatus, Session, Source};
use wiremock::matchers::{header_exists, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const KONYAKU: &str = r#"{"manga_list":[{"id":32807,"title":"Konyaku Hakisarete"}]}"#;
const KONYAKU_DETAIL: &str = r#"{"manga":{"id":32807,"title":"Konyaku Hakisarete"}}"#;
/// As the API ranks it: best matches first, then fuzzy ones that do not contain the words.
const RANKED: &str = r#"{"manga_list":[{"id":115,"title":"BERSERK"},{"id":7,"title":"Gaiden"},{"id":9,"title":"Bersaga"}]}"#;

fn session() -> Session {
    Session {
        cookies: "cf_clearance=abc".into(),
        user_agent: "UA".into(),
        saved_at: None,
    }
}

/// A fresh server (own port, so its own rate-limit bucket) guarding `/api` like the site does.
async fn edge() -> MockServer {
    let server = MockServer::builder().start().await;
    Mock::given(path_regex("^/api/"))
        .respond_with(challenge_page())
        .with_priority(u8::MAX)
        .mount(&server)
        .await;
    server
}

async fn api(server: &MockServer, route: &str, query: Option<(&str, &str)>, body: &str) {
    let mut mock = Mock::given(method("GET"))
        .and(path(route))
        .and(header_exists("referer"));
    if let Some((name, value)) = query {
        mock = mock.and(query_param(name, value));
    }
    mock.respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(server)
        .await;
}

fn source(server: &MockServer, session: Option<Session>) -> MangaDot {
    MangaDot::with_base(&format!("{}/api", server.uri()), session).unwrap()
}

/// Headers of every request received on `route`.
async fn sent(server: &MockServer, route: &str) -> Vec<wiremock::Request> {
    let requests = server.received_requests().await.unwrap_or_default();
    requests
        .into_iter()
        .filter(|r| r.url.path() == route)
        .collect()
}

fn header<'a>(request: &'a wiremock::Request, name: &str) -> Option<&'a str> {
    request.headers.get(name).and_then(|v| v.to_str().ok())
}

#[tokio::test]
async fn api_requests_pass_the_referer_rule_with_same_origin_fetch_headers() {
    let server = edge().await;
    api(&server, "/api/search", None, KONYAKU).await;
    api(&server, "/api/manga/32807", None, KONYAKU_DETAIL).await;
    let source = source(&server, Some(session()));
    let found = source.search("konyaku", 5).await.unwrap();
    assert_eq!(found[0].id, MangaId("32807".into()));
    let manga = source.manga(&MangaId("32807".into())).await.unwrap();
    assert_eq!(manga.title, "Konyaku Hakisarete");

    let search = &sent(&server, "/api/search").await[0];
    assert_eq!(header(search, "referer"), Some("https://mangadot.net/"));
    assert_eq!(header(search, "origin"), Some("https://mangadot.net"));
    assert_eq!(header(search, "sec-fetch-site"), Some("same-origin"));
    assert_eq!(header(search, "sec-fetch-mode"), Some("cors"));
    assert_eq!(header(search, "sec-fetch-dest"), Some("empty"));
    assert_eq!(header(search, "cookie"), Some("cf_clearance=abc"));
    assert_eq!(header(search, "user-agent"), Some("UA"));

    let detail = &sent(&server, "/api/manga/32807").await[0];
    assert_eq!(header(detail, "origin"), Some("https://mangadot.net"));
    assert_eq!(header(detail, "sec-fetch-mode"), Some("cors"));
}

#[tokio::test]
async fn status_probe_is_ready_with_a_session_and_needs_one_without() {
    let server = edge().await;
    api(&server, "/api/manga", Some(("limit", "1")), KONYAKU).await;
    assert_eq!(
        source(&server, Some(session())).status().await,
        ProviderStatus::Ready
    );
    assert_eq!(sent(&server, "/api/manga").await.len(), 1);
    assert_eq!(
        source(&server, None).status().await,
        ProviderStatus::NeedsSession
    );
    assert_eq!(
        sent(&server, "/api/manga").await.len(),
        1,
        "probed without a session"
    );
}

#[tokio::test]
async fn a_request_without_the_referer_still_reads_as_a_challenge() {
    let server = edge().await;
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/search?search=x", server.uri()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 403);
    assert!(response.text().await.unwrap().contains("Just a moment..."));
}

#[tokio::test]
async fn search_keeps_the_api_ranking_without_filtering_or_walking_the_catalogue() {
    let server = edge().await;
    api(&server, "/api/search", Some(("search", "berserk")), RANKED).await;
    let found = source(&server, Some(session()))
        .search("berserk", 2)
        .await
        .unwrap();
    let ids: Vec<_> = found.iter().map(|m| m.id.0.as_str()).collect();
    assert_eq!(ids, ["115", "7"]);
    let search = &sent(&server, "/api/search").await;
    assert_eq!(search.len(), 1);
    let limit = search[0].url.query_pairs().find(|(k, _)| k == "limit");
    assert_eq!(limit.map(|(_, v)| v.into_owned()).as_deref(), Some("2"));
    assert!(sent(&server, "/api/manga").await.is_empty());
}
