use mangas_providers::http::{host_matches, is_api_path, HostPolicy};
use mangas_providers::Session;

fn session() -> Session {
    Session {
        cookies: "cf_clearance=abc".into(),
        user_agent: "UA".into(),
        saved_at: None,
    }
}

fn names(headers: Vec<(String, String)>) -> Vec<String> {
    headers.into_iter().map(|(name, _)| name).collect()
}

#[test]
fn site_and_subdomains_get_referer_and_session() {
    let policy = HostPolicy::new("https://example.com", &[], Some(session()));
    let headers = policy.headers_for("https://example.com/img/1.webp");
    assert_eq!(
        headers[0],
        ("referer".to_string(), "https://example.com/".to_string())
    );
    assert_eq!(names(headers), ["referer", "cookie", "user-agent"]);
    assert_eq!(
        names(policy.headers_for("https://img.example.com/1.webp")),
        ["referer", "cookie", "user-agent"]
    );
}

#[test]
fn session_never_leaves_the_site() {
    let policy = HostPolicy::new("https://example.com", &["cdn.net"], Some(session()));
    assert_eq!(
        names(policy.headers_for("https://a.cdn.net/1.webp")),
        ["referer"]
    );
    assert!(policy.headers_for("https://other.org/1.webp").is_empty());
    assert!(policy
        .headers_for("https://example.com.evil.io/1.webp")
        .is_empty());
    assert!(policy
        .headers_for("https://notexample.com/1.webp")
        .is_empty());
}

#[test]
fn plaintext_requests_to_an_https_site_get_no_session() {
    let policy = HostPolicy::new("https://example.com", &[], Some(session()));
    assert!(policy.headers_for("http://example.com/1.webp").is_empty());
}

#[test]
fn wildcard_image_hosts_get_the_referer_everywhere() {
    let policy = HostPolicy::new("https://example.com", &["*"], Some(session()));
    assert_eq!(
        names(policy.headers_for("https://any.cdn/1.webp")),
        ["referer"]
    );
    assert!(policy.headers_for("http://any.cdn/1.webp").is_empty());
}

#[test]
fn without_session_the_site_still_gets_the_referer() {
    let policy = HostPolicy::new("https://example.com/", &[], None);
    assert_eq!(
        policy.headers_for("https://example.com/1.webp"),
        vec![("referer".to_string(), "https://example.com/".to_string())]
    );
    assert!(policy.headers_for("not a url").is_empty());
}

#[test]
fn host_matching_is_exact_or_subdomain() {
    assert!(host_matches("example.com", "example.com"));
    assert!(host_matches("a.b.example.com", ".example.com"));
    assert!(host_matches("EXAMPLE.com", "example.COM"));
    assert!(!host_matches("badexample.com", "example.com"));
    assert!(!host_matches("example.com", ""));
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
}

#[test]
fn every_request_to_the_base_carries_the_site_referer() {
    let policy = HostPolicy::new("https://example.com/", &[], Some(session()));
    let page = policy.request_headers("https://example.com/manga/1");
    assert_eq!(
        page,
        vec![("referer".to_string(), "https://example.com/".to_string())]
    );
    let other_host = policy.request_headers("https://m.example.org/list");
    assert_eq!(names(other_host), ["referer"]);
}

#[test]
fn api_requests_look_like_the_sites_own_fetches() {
    let policy = HostPolicy::new("https://example.com", &[], Some(session()));
    let headers = policy.request_headers("https://example.com/api/search?search=x");
    assert_eq!(header(&headers, "referer"), Some("https://example.com/"));
    assert_eq!(header(&headers, "origin"), Some("https://example.com"));
    assert_eq!(header(&headers, "sec-fetch-site"), Some("same-origin"));
    assert_eq!(header(&headers, "sec-fetch-mode"), Some("cors"));
    assert_eq!(header(&headers, "sec-fetch-dest"), Some("empty"));
    assert_eq!(
        header(&headers, "cookie"),
        None,
        "cookies come from the client"
    );
}

#[test]
fn without_session_api_requests_get_no_fetch_metadata() {
    let policy = HostPolicy::new("https://example.com", &[], None);
    let headers = policy.request_headers("https://example.com/api/manga?page=1");
    assert_eq!(names(headers), ["referer", "origin"]);
}

#[test]
fn api_paths_are_the_api_segment_and_below() {
    assert!(is_api_path("https://example.com/api"));
    assert!(is_api_path("https://example.com/api/comics/x/chapter-list"));
    assert!(is_api_path("http://127.0.0.1:8080/api/search?q=a"));
    assert!(!is_api_path("https://example.com/apis/x"));
    assert!(!is_api_path("https://example.com/manga/api"));
    assert!(!is_api_path("https://api.example.com/manga"));
    assert!(!is_api_path("not a url"));
}

#[test]
fn another_port_on_the_site_host_is_another_service() {
    let policy = HostPolicy::new("https://example.com", &[], Some(session()));
    assert!(policy
        .headers_for("https://example.com:8443/1.webp")
        .is_empty());
    assert_eq!(
        names(policy.headers_for("https://example.com:443/1.webp")),
        ["referer", "cookie", "user-agent"]
    );
}
