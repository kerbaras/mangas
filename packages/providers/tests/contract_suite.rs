//! Offline contract suite: every registered provider, built at a mock server answering with
//! its recorded fixtures (`tests/fixtures/<id>/contract/`).

mod contract;

use contract::{headers, manifest, walk};
use mangas_core::domain::{Error, MangaId};
use mangas_core::library::Settings;
use mangas_providers::{providers, source_at, ProviderStatus};
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn every_provider_has_a_contract_manifest() {
    for info in providers() {
        let manifest = manifest::load(info.id);
        let dir = manifest::dir(info.id);
        assert!(
            manifest.is_some(),
            "{}: add {}/manifest.json",
            info.id,
            dir.display()
        );
    }
}

#[tokio::test]
async fn providers_walk_search_to_pages_on_fixtures() {
    for info in providers() {
        let Some(manifest) = manifest::load(info.id) else {
            continue;
        };
        let server = manifest::serve(info.id, &manifest).await;
        let settings = contract::with_session(info.id);
        let source = source_at(info.id, &settings, &server.uri()).unwrap();
        let walked = walk::assert_walk(&*source, &info, &manifest.query, &manifest.language).await;
        let session = settings.provider_sessions.get(info.id);
        let need_referer = manifest.pages_need_referer;
        headers::assert_page_headers(&*source, &info, &walked.pages, need_referer, session);
    }
}

/// A server answering everything with `response`.
async fn answering(response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

fn challenge_page(header: bool) -> ResponseTemplate {
    let page = ResponseTemplate::new(403).set_body_string(
        "<title>Just a moment...</title><script src=\"/cdn-cgi/challenge-platform/x\">",
    );
    if header {
        page.insert_header("cf-mitigated", "challenge")
    } else {
        page
    }
}

#[tokio::test]
async fn challenge_pages_become_challenge_errors() {
    for header in [true, false] {
        let server = answering(challenge_page(header)).await;
        for info in providers() {
            let settings = contract::with_session(info.id);
            let source = source_at(info.id, &settings, &server.uri()).unwrap();
            let id = MangaId("1".into());
            let results = [
                source.search("x", 5).await.err(),
                source.manga(&id).await.err(),
                source.chapters(&id, info.languages[0]).await.err(),
            ];
            for err in results {
                let ok = matches!(&err, Some(Error::Challenge { name, .. }) if name == info.id);
                assert!(ok, "{}: expected a challenge, got {err:?}", info.id);
            }
        }
    }
}

#[tokio::test]
async fn status_probe_maps_responses() {
    let cases = [
        (ResponseTemplate::new(200).set_body_string("{}"), "ready"),
        (challenge_page(true), "needs_session"),
        (ResponseTemplate::new(500), "unreachable"),
    ];
    for (response, expected) in cases {
        let server = answering(response).await;
        for info in providers() {
            let settings = contract::with_session(info.id);
            let source = source_at(info.id, &settings, &server.uri()).unwrap();
            let status = source.status().await;
            assert_eq!(state(&status), expected, "{}: {status:?}", info.id);
        }
    }
}

/// #219: a missing required session decides the status only once the host answers; a host
/// that can't be reached (or fails with a 5xx) is unreachable, so an outage isn't hidden behind
/// "needs session".
#[tokio::test]
async fn missing_required_session_needs_session_only_when_the_host_answers() {
    let server = answering(ResponseTemplate::new(200)).await;
    let failing = answering(ResponseTemplate::new(500)).await;
    let dead = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead = format!("http://{}", dead.local_addr().unwrap());
    for info in providers().into_iter().filter(|p| p.needs_browser_session) {
        let source = source_at(info.id, &Settings::default(), &server.uri()).unwrap();
        let status = source.status().await;
        assert_eq!(status, ProviderStatus::NeedsSession, "{}", info.id);
        for base in [failing.uri(), dead.clone()] {
            let source = source_at(info.id, &Settings::default(), &base).unwrap();
            let status = source.status().await;
            assert!(status.is_unreachable(), "{} at {base}: {status:?}", info.id);
        }
    }
}

fn state(status: &ProviderStatus) -> &'static str {
    match status {
        ProviderStatus::Ready => "ready",
        ProviderStatus::NeedsSession => "needs_session",
        ProviderStatus::Unreachable { .. } => "unreachable",
    }
}
