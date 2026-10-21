//! A TLS server on localhost with a self-signed certificate; counts the connections it accepts.
//!
//! `fixtures/tls/` holds a test-only, self-signed certificate and key for `localhost` (served on
//! 127.0.0.1). Only this test helper reads them; nothing outside `tests/` does.

use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

const CERT: &[u8] = include_bytes!("fixtures/tls/cert.der");
const KEY: &[u8] = include_bytes!("fixtures/tls/key.der");

#[allow(dead_code)]
pub fn start() -> (String, Arc<AtomicUsize>) {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(KEY.to_vec()));
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![CertificateDer::from(CERT.to_vec())], key)
        .unwrap();
    let config = Arc::new(config);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let accepted = Arc::new(AtomicUsize::new(0));
    let count = accepted.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            count.fetch_add(1, Ordering::SeqCst);
            let Ok(mut conn) = rustls::ServerConnection::new(config.clone()) else {
                continue;
            };
            while conn.complete_io(&mut stream).is_ok() && conn.is_handshaking() {}
        }
    });
    (format!("https://localhost:{port}/"), accepted)
}
