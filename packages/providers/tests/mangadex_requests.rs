//! MangaDex request shapes and the paginated feed walk against a local server.

use mangas_core::domain::MangaId;
use mangas_core::sources::Source;
use mangas_providers::mangadex::{feed_query, search_query, MangaDex, MangaDexOptions};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const EMINENCE: &str = "91d928a1-7237-4d03-abc3-4b1440cc7d55";

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/mangadex/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn id() -> MangaId {
    MangaId(EMINENCE.into())
}

#[test]
fn search_filters_by_rating_and_language_and_includes_covers() {
    let languages = vec!["en".to_string(), "es-la".to_string()];
    let q = search_query("solo", 500, &["safe", "suggestive"], &languages);
    let has = |k: &str, v: &str| q.iter().any(|(qk, qv)| *qk == k && qv == v);
    assert!(has("limit", "100"), "clamped to the API max");
    assert!(has("includes[]", "cover_art"));
    assert!(has("includes[]", "author") && has("includes[]", "artist"));
    assert!(has("contentRating[]", "safe") && has("contentRating[]", "suggestive"));
    assert!(!has("contentRating[]", "erotica"));
    assert!(has("availableTranslatedLanguage[]", "en"));
    assert!(has("availableTranslatedLanguage[]", "es-la"));
}

#[test]
fn feed_asks_for_groups_and_every_rating() {
    let q = feed_query("en", 500);
    let has = |k: &str, v: &str| q.iter().any(|(qk, qv)| *qk == k && qv == v);
    assert!(has("includes[]", "scanlation_group"));
    assert!(has("offset", "500") && has("limit", "500"));
    assert!(has("contentRating[]", "pornographic"));
}

/// Serves the two-page feed fixture; `edge_errors` HTML `400`s come first on page 2.
async fn feed_server(edge_errors: u64) -> MockServer {
    let server = MockServer::start().await;
    let feed = format!("/manga/{EMINENCE}/feed");
    if edge_errors > 0 {
        let page = "<!doctype html><title>MangaDex</title>400: Bad Request";
        Mock::given(path(feed.as_str()))
            .and(query_param("offset", "6"))
            .respond_with(ResponseTemplate::new(400).set_body_raw(page, "text/html"))
            .up_to_n_times(edge_errors)
            .expect(edge_errors)
            .mount(&server)
            .await;
    }
    for (offset, file) in [
        ("0", "feed_groups_page1.json"),
        ("6", "feed_groups_page2.json"),
    ] {
        Mock::given(path(feed.as_str()))
            .and(query_param("offset", offset))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture(file)))
            .expect(1)
            .mount(&server)
            .await;
    }
    server
}

async fn chapter_numbers(server: &MockServer) -> Vec<String> {
    let options = MangaDexOptions {
        preferred_groups: vec!["Mitsugoshi".into()],
        ..MangaDexOptions::default()
    };
    let source = MangaDex::with_base(&server.uri())
        .unwrap()
        .with_preferences(options, vec!["en".into()]);
    let chapters = source.chapters(&id(), "en").await.unwrap();
    assert_eq!(
        chapters.len(),
        12,
        "every group's upload is kept; core picks among them"
    );
    let mitsugoshi = chapters
        .iter()
        .filter(|c| c.group.as_deref() == Some("Mitsugoshi"));
    assert_eq!(mitsugoshi.count(), 5);
    let mut numbers: Vec<String> = chapters.into_iter().map(|c| c.number).collect();
    numbers.dedup();
    numbers
}

const NUMBERS: [&str; 6] = ["1", "2.1", "2.2", "3.1", "3.2", "4.1"];

#[tokio::test]
async fn chapters_walk_every_feed_page_and_keep_every_group() {
    let server = feed_server(0).await;
    assert_eq!(chapter_numbers(&server).await, NUMBERS);
}

#[tokio::test]
async fn feed_page_survives_one_edge_error_page() {
    let server = feed_server(1).await;
    assert_eq!(chapter_numbers(&server).await, NUMBERS);
}
