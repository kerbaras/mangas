//! A refused server certificate becomes `Error::Certificate`, also through a configured proxy.

mod tls_server;

use mangas_core::domain::{CertProblem, Error};
use mangas_core::library::Network;
use mangas_core::net::transport_error;

async fn refused() -> reqwest::Error {
    let (url, _) = tls_server::start();
    reqwest::get(url).await.unwrap_err()
}

#[tokio::test]
async fn a_self_signed_site_names_its_certificate() {
    if !mangas_core::domain::system_has_root_certificates() {
        return;
    }
    let error = Error::from(refused().await);
    assert!(
        matches!(
            &error,
            Error::Certificate { host, problem: CertProblem::UnknownIssuer } if host == "localhost"
        ),
        "{error}"
    );
    let proxy = Network::Proxy("http://127.0.0.1:9".into());
    let through_proxy = transport_error(refused().await, &proxy);
    assert!(
        matches!(through_proxy, Error::Certificate { .. }),
        "{through_proxy}"
    );
}
