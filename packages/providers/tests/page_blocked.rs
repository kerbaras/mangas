//! A page host whose name resolves to a non-public address, or that is a private IP literal or
//! `localhost`, is refused once, with a message that says so, instead of being retried.

use std::time::{Duration, Instant};

use mangas_core::domain::Error;
use mangas_core::download::PageFetcher;
use mangas_core::library::Settings;
use mangas_providers::http::PageClient;

#[tokio::test]
async fn a_name_resolving_to_a_private_address_is_refused_at_once() {
    let source = mangas_providers::source("mangadex", &Settings::default()).unwrap();
    for ip in [
        "127.0.0.1",
        "10.1.2.3",
        "::ffff:127.0.0.1",
        "169.254.169.254",
    ] {
        let client = PageClient::for_source(source.as_ref())
            .and_then(|c| c.resolving_to(ip.parse().unwrap()))
            .unwrap();
        let started = Instant::now();
        let err = client.fetch("https://cdn.qa.test/x.jpg").await.unwrap_err();
        // PAGE_RETRY waits 1 s before its first retry: anything faster made one attempt.
        assert!(started.elapsed() < Duration::from_secs(1), "{ip}: retried");
        assert!(matches!(err, Error::BlockedAddress { .. }), "{err:?}");
        let ip = mangas_core::download::unmapped(ip.parse().unwrap());
        let expected = format!("refused: cdn.qa.test resolves to a non-public address {ip}");
        assert_eq!(err.to_string(), expected);
    }
}

#[tokio::test]
async fn a_private_ip_literal_or_localhost_is_refused_at_once() {
    let source = mangas_providers::source("mangadex", &Settings::default()).unwrap();
    let client = PageClient::for_source(source.as_ref()).unwrap();
    for (url, host, ip) in [
        ("http://127.0.0.1:9/1.png", "127.0.0.1", "127.0.0.1"),
        ("https://10.0.0.5/x.jpg", "10.0.0.5", "10.0.0.5"),
        (
            "http://169.254.169.254/latest",
            "169.254.169.254",
            "169.254.169.254",
        ),
        ("http://[::1]/x.jpg", "[::1]", "::1"),
        ("http://[::ffff:10.0.0.1]/x.jpg", "10.0.0.1", "10.0.0.1"),
        ("http://localhost:8080/x.jpg", "localhost", "127.0.0.1"),
        ("http://cdn.localhost/x.jpg", "cdn.localhost", "127.0.0.1"),
    ] {
        let started = Instant::now();
        let err = client.fetch(url).await.unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(1), "{url}: retried");
        assert!(
            matches!(err, Error::BlockedAddress { .. }),
            "{url}: {err:?}"
        );
        let expected = format!("refused: {host} resolves to a non-public address {ip}");
        assert_eq!(err.to_string(), expected);
    }
}

#[test]
fn a_refused_address_is_not_retried_by_the_queue() {
    use mangas_core::download::queue::classify;
    let err = Error::BlockedAddress {
        host: "localhost".into(),
        ip: "127.0.0.1".parse().unwrap(),
    };
    assert!(!classify(&err).retryable);
}

#[test]
fn an_ipv4_mapped_address_reads_as_ipv4() {
    use mangas_core::download::unmapped;
    let shown = |ip: &str| unmapped(ip.parse().unwrap()).to_string();
    assert_eq!(shown("::ffff:10.0.0.1"), "10.0.0.1");
    assert_eq!(shown("::ffff:127.0.0.1"), "127.0.0.1");
    assert_eq!(shown("fd00::1"), "fd00::1");
    assert_eq!(shown("::1"), "::1");
    assert_eq!(shown("10.0.0.1"), "10.0.0.1");
}
