//! `tests/fixtures/<id>/contract/manifest.json`: what to search for and which recorded bodies
//! the mock server answers with, by path.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;
use wiremock::matchers::{header_exists, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub query: String,
    pub language: String,
    /// Page images of this provider are fetched with the site's referer.
    pub pages_need_referer: bool,
    /// Path prefix the site's edge guards: without a `Referer` it answers the challenge page.
    #[serde(default)]
    pub challenge_without_referer: Option<String>,
    #[serde(default)]
    pub routes: Vec<Route>,
}

/// One GET the provider makes: path relative to the base it was built at, query parameters
/// that must be present, and the fixture file answering it.
#[derive(Debug, Deserialize)]
pub struct Route {
    pub path: String,
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    pub file: String,
}

pub fn dir(id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(id)
        .join("contract")
}

/// The manifest of `id`, or `None` when the provider has none yet.
pub fn load(id: &str) -> Option<Manifest> {
    let text = std::fs::read_to_string(dir(id).join("manifest.json")).ok()?;
    Some(serde_json::from_str(&text).unwrap_or_else(|e| panic!("{id}: bad manifest: {e}")))
}

/// A mock server answering every route of `manifest` with its fixture.
pub async fn serve(id: &str, manifest: &Manifest) -> MockServer {
    let server = MockServer::start().await;
    let guarded = manifest.challenge_without_referer.as_deref();
    for route in &manifest.routes {
        let file = dir(id).join(&route.file);
        let body = std::fs::read(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let mut mock = Mock::given(method("GET")).and(path(route.path.as_str()));
        for (name, value) in &route.query {
            mock = mock.and(query_param(name.as_str(), value.as_str()));
        }
        if guarded.is_some_and(|prefix| route.path.starts_with(prefix)) {
            mock = mock.and(header_exists("referer"));
        }
        let response = ResponseTemplate::new(200).set_body_raw(body, content_type(&route.file));
        mock.respond_with(response).mount(&server).await;
    }
    if let Some(prefix) = guarded {
        Mock::given(path_regex(format!("^{prefix}")))
            .respond_with(challenge_page())
            .with_priority(u8::MAX)
            .mount(&server)
            .await;
    }
    server
}

/// Cloudflare's interstitial, as its edge serves it to a request it does not trust.
pub fn challenge_page() -> ResponseTemplate {
    ResponseTemplate::new(403)
        .insert_header("cf-mitigated", "challenge")
        .set_body_raw(
            "<title>Just a moment...</title><script src=\"/cdn-cgi/challenge-platform/x\">",
            "text/html; charset=utf-8",
        )
}

fn content_type(file: &str) -> &'static str {
    match file.rsplit('.').next() {
        Some("json") => "application/json",
        Some("html") => "text/html; charset=utf-8",
        _ => "application/octet-stream",
    }
}
