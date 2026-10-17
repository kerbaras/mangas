use mangas_providers::http::{host_matches, HostPolicy};
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
