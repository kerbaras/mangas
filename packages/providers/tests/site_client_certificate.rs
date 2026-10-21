//! A refused certificate fails at once: no transport retries against the same certificate.

#[path = "../../core/tests/tls_server.rs"]
mod tls_server;

use std::sync::atomic::Ordering;
use std::time::Duration;

use mangas_core::domain::Error;
use mangas_providers::http::{ClientConfig, RetryPolicy, SiteClient};
use mangas_providers::mangadot;

#[tokio::test]
async fn a_self_signed_site_is_tried_once() {
    if !mangas_core::domain::system_has_root_certificates() {
        return;
    }
    let (url, accepted) = tls_server::start();
    let config = ClientConfig {
        retry: RetryPolicy {
            max_retries: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(5),
        },
        requests_per_second: 1000,
        ..ClientConfig::new(mangadot::INFO, url.trim_end_matches('/'))
    };
    let error = SiteClient::new(config, None)
        .unwrap()
        .get("/ping", &[])
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Certificate { .. }), "{error}");
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
}
