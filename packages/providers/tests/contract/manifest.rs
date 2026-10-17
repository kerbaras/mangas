//! `tests/fixtures/<id>/contract/manifest.json`: what to search for and which recorded bodies
//! the mock server answers with, by path.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub query: String,
    pub language: String,
    /// Page images of this provider are fetched with the site's referer.
    pub pages_need_referer: bool,
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
    for route in &manifest.routes {
        let file = dir(id).join(&route.file);
        let body = std::fs::read(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let mut mock = Mock::given(method("GET")).and(path(route.path.as_str()));
        for (name, value) in &route.query {
            mock = mock.and(query_param(name.as_str(), value.as_str()));
        }
        let response = ResponseTemplate::new(200).set_body_raw(body, content_type(&route.file));
        mock.respond_with(response).mount(&server).await;
    }
    server
}

fn content_type(file: &str) -> &'static str {
    match file.rsplit('.').next() {
        Some("json") => "application/json",
        Some("html") => "text/html; charset=utf-8",
        _ => "application/octet-stream",
    }
}
