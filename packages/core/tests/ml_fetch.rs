//! Detector downloads stop on Cancel even when the server stalls.

use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use mangas_core::ml::fetch::fetch;
use mangas_core::ml::is_cancelled;

#[tokio::test]
async fn cancel_stops_a_download_the_server_never_answers() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/ort.tgz", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let _held: Vec<_> = listener.incoming().take(1).collect();
        std::thread::sleep(Duration::from_secs(30));
    });
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        flag.store(true, Ordering::SeqCst);
    });
    let mut ignore = |_, _| {};
    let network = Default::default();
    let run = fetch(&url, &network, &mut ignore, &cancel);
    let err = tokio::time::timeout(Duration::from_secs(5), run)
        .await
        .expect("Cancel must not wait for the stalled server")
        .unwrap_err();
    assert!(is_cancelled(&err), "{err}");
}

/// Detector downloads go through the stored proxy like provider traffic does.
#[tokio::test]
async fn a_download_goes_through_the_configured_proxy() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let proxy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let network =
        mangas_core::library::Network::Proxy(format!("http://{}", proxy.local_addr().unwrap()));
    let seen = tokio::spawn(async move {
        let (mut conn, _) = proxy.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = conn.read(&mut buf).await.unwrap();
        conn.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nmodel")
            .await
            .unwrap();
        String::from_utf8_lossy(&buf[..n])
            .lines()
            .next()
            .unwrap()
            .to_string()
    });
    let never = AtomicBool::new(false);
    let mut ignore = |_, _| {};
    let body = fetch(
        "http://models.invalid/m.onnx",
        &network,
        &mut ignore,
        &never,
    )
    .await
    .unwrap();
    assert_eq!(body, b"model");
    assert_eq!(
        seen.await.unwrap(),
        "GET http://models.invalid/m.onnx HTTP/1.1"
    );
}
