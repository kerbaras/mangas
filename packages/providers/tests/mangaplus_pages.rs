//! MANGA Plus pages: the viewer's page URLs, and page images decrypted on the real download
//! path (`PageClient`) with the viewer token sent.

use mangas_core::domain::Error;
use mangas_core::download::PageFetcher;
use mangas_core::library::Settings;
use mangas_providers::http::PageClient;
use mangas_providers::mangaplus::{page_url, parse_viewer, view_token, xor};
use wiremock::matchers::{header, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fixture(name: &str) -> Vec<u8> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/mangaplus/");
    std::fs::read(format!("{dir}{name}")).unwrap()
}

#[test]
fn viewer_pages_carry_their_key_and_the_view_token() {
    let pages = parse_viewer(&fixture("contract/manga_viewer.pb"))
        .unwrap()
        .0;
    assert_eq!(pages.len(), 51);
    for page in &pages {
        assert!(page.starts_with("https://jumpg-assets"), "{page}");
        assert!(
            page.contains("#key=") && view_token(page).is_some(),
            "{page}"
        );
    }
    let source = mangas_providers::source("mangaplus", &Settings::default()).unwrap();
    let headers = source.page_headers(&pages[0]);
    let token = headers.iter().find(|(name, _)| name == "Plus-Vw-Token");
    assert_eq!(token.map(|(_, v)| v.as_str()), view_token(&pages[0]));
    let elsewhere = source.page_headers("https://images.example/1.jpg#key=00&vw=tok");
    assert!(
        elsewhere.iter().all(|(name, _)| name != "Plus-Vw-Token"),
        "{elsewhere:?}"
    );
}

#[tokio::test]
async fn a_rejection_popup_is_unreachable() {
    let server = MockServer::start().await;
    Mock::given(path("/title_detailV3"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(fixture("banned.pb")))
        .mount(&server)
        .await;
    let source = mangas_providers::source_at("mangaplus", &Settings::default(), &server.uri());
    let status = source.unwrap().status().await;
    assert!(status.to_string().contains("Account Banned"), "{status}");
}

/// The recorded last page of chapter 1 (`page.enc`) with its key (`page.key`).
fn encrypted_page() -> (Vec<u8>, String) {
    let key = String::from_utf8(fixture("page.key")).unwrap();
    (fixture("page.enc"), key.trim().to_string())
}

fn hex(key: &str) -> Vec<u8> {
    (0..key.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&key[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn xor_with_the_page_key_gives_the_jpeg() {
    let (mut bytes, key) = encrypted_page();
    assert_ne!(bytes[..3], [0xff, 0xd8, 0xff]);
    xor(&mut bytes, &hex(&key));
    assert_eq!(bytes[..3], [0xff, 0xd8, 0xff], "JPEG start");
    assert_eq!(bytes[bytes.len() - 2..], [0xff, 0xd9], "JPEG end");
}

#[tokio::test]
async fn the_page_client_sends_the_token_and_decrypts() {
    let (encrypted, key) = encrypted_page();
    let server = MockServer::start().await;
    Mock::given(path("/secure/51.jpg"))
        .and(header("plus-vw-token", "tok123"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(encrypted.clone()))
        .mount(&server)
        .await;
    let source = mangas_providers::source_at("mangaplus", &Settings::default(), &server.uri());
    let source = source.unwrap();
    let client = PageClient::for_source(source.as_ref())
        .and_then(|c| c.allow_private_hosts())
        .unwrap();
    let url = page_url(
        &format!("{}/secure/51.jpg?hash=h", server.uri()),
        &key,
        "tok123",
    );
    let body = client.fetch(&url).await.unwrap();
    let mut expected = encrypted;
    xor(&mut expected, &hex(&key));
    assert_eq!(body, expected);
    let requested = server.received_requests().await.unwrap();
    let sent = &requested[0].url;
    assert_eq!(
        (sent.path(), sent.query()),
        ("/secure/51.jpg", Some("hash=h"))
    );
    assert_eq!(
        sent.fragment(),
        None,
        "the key and token never go on the wire"
    );
}

#[tokio::test]
async fn a_page_without_the_token_is_an_http_error() {
    let server = MockServer::start().await;
    let source = mangas_providers::source("mangaplus", &Settings::default()).unwrap();
    let client = PageClient::for_source(source.as_ref())
        .and_then(|c| c.allow_private_hosts())
        .unwrap()
        .with_retry(mangas_providers::http::RetryPolicy::NONE);
    let err = client
        .fetch(&format!("{}/secure/1.jpg", server.uri()))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Source { .. }), "{err:?}");
}
