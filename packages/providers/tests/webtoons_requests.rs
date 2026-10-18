//! Webtoons request flow against a mock server: canvas search fallbacks, languages, paging.

use mangas_core::domain::MangaId;
use mangas_core::library::Settings;
use mangas_providers::source_at;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SEARCH: &str = r#"{"result":{"searchedList":[{"titleNo":95,"title":"Tower of God"}]}}"#;

fn settings(languages: &[&str]) -> Settings {
    Settings {
        preferred_languages: languages.iter().map(|l| l.to_string()).collect(),
        ..Settings::default()
    }
}

async fn reply(server: &MockServer, at: &str, status: u16, body: &str) {
    Mock::given(method("GET"))
        .and(path(at))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(server)
        .await;
}

#[tokio::test]
async fn search_covers_every_supported_language_and_tolerates_missing_canvas() {
    let server = MockServer::start().await;
    reply(&server, "/en/search/immediate", 200, SEARCH).await;
    reply(&server, "/en/search/canvas", 404, "").await;
    reply(&server, "/es/search/immediate", 200, SEARCH).await;
    let canvas = r#"<a data-title-no="7" data-webtoon-type="CHALLENGE"><strong class="title">Mi torre</strong></a>"#;
    reply(&server, "/es/search/canvas", 200, canvas).await;
    let source = source_at("webtoons", &settings(&["en", "ja", "es"]), &server.uri()).unwrap();
    let found = source.search("tower", 10).await.unwrap();
    let ids: Vec<_> = found.iter().map(|m| m.id.0.as_str()).collect();
    assert_eq!(ids, ["en-webtoon-95", "es-webtoon-95", "es-canvas-7"]);
    assert_eq!(source.search("tower", 1).await.unwrap().len(), 1);
}

#[tokio::test]
async fn failing_canvas_search_keeps_the_originals() {
    let server = MockServer::start().await;
    reply(&server, "/en/search/immediate", 200, SEARCH).await;
    reply(&server, "/en/search/canvas", 400, "").await;
    let source = source_at("webtoons", &settings(&["en"]), &server.uri()).unwrap();
    assert_eq!(source.search("tower", 10).await.unwrap().len(), 1);
}

#[tokio::test]
async fn chapters_in_another_language_need_no_request() {
    let server = MockServer::start().await;
    let source = source_at("webtoons", &Settings::default(), &server.uri()).unwrap();
    let id = MangaId("en-webtoon-95".into());
    assert!(source.chapters(&id, "es").await.unwrap().is_empty());
    let requests = server.received_requests().await.unwrap_or_default();
    assert!(requests.is_empty(), "{requests:?}");
}

fn episodes(from: u64, to: u64, next: u64) -> String {
    let list: Vec<String> = (from..=to)
        .map(|n| format!(r#"{{"episodeNo":{n},"episodeTitle":"Ep. {n}"}}"#))
        .collect();
    format!(
        r#"{{"result":{{"episodeList":[{}],"nextCursor":{next}}}}}"#,
        list.join(",")
    )
}

#[tokio::test]
async fn canvas_chapters_follow_the_cursor_with_the_reading_language() {
    let server = MockServer::start().await;
    let at = "/api/v1/canvas/7/episodes";
    let second = Mock::given(method("GET"))
        .and(path(at))
        .and(query_param("cursor", "1001"))
        .and(query_param("readingLanguageCode", "es"));
    second
        .respond_with(ResponseTemplate::new(200).set_body_string(episodes(1001, 1002, 0)))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(at))
        .and(query_param("readingLanguageCode", "es"))
        .respond_with(ResponseTemplate::new(200).set_body_string(episodes(1, 1000, 1001)))
        .mount(&server)
        .await;
    let source = source_at("webtoons", &Settings::default(), &server.uri()).unwrap();
    let chapters = source
        .chapters(&MangaId("es-canvas-7".into()), "es")
        .await
        .unwrap();
    assert_eq!(chapters.len(), 1002);
    assert_eq!(chapters[1001].number, "1002");
}

#[tokio::test]
async fn unknown_titles_are_not_found() {
    let server = MockServer::start().await;
    reply(
        &server,
        "/api/v1/webtoon/1/episodes",
        200,
        r#"{"result":null}"#,
    )
    .await;
    let source = source_at("webtoons", &Settings::default(), &server.uri()).unwrap();
    let err = source
        .chapters(&MangaId("1".into()), "en")
        .await
        .unwrap_err();
    assert!(
        matches!(err, mangas_core::domain::Error::NotFound(_)),
        "{err:?}"
    );
}
