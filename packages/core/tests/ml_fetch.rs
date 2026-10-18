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
    let run = fetch(&url, &mut ignore, &cancel);
    let err = tokio::time::timeout(Duration::from_secs(5), run)
        .await
        .expect("Cancel must not wait for the stalled server")
        .unwrap_err();
    assert!(is_cancelled(&err), "{err}");
}
