//! Email delivery against a local SMTP double (plain connection, no login).

use std::sync::{Arc, Mutex};

use mangas_core::send::email::{parse_sender, send_book, send_message, send_test, test_message};
use mangas_core::send::{EmailSettings, Security};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

/// Accepts one session and records what the client sent; answers the end of DATA with
/// `final_reply`.
async fn smtp_double(final_reply: &'static str) -> (u16, Arc<Mutex<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(String::new()));
    let seen = log.clone();
    tokio::spawn(async move {
        let (sock, _) = listener.accept().await.unwrap();
        let (read, mut write) = sock.into_split();
        let mut lines = BufReader::new(read).lines();
        write.write_all(b"220 double ESMTP\r\n").await.unwrap();
        let mut in_data = false;
        while let Ok(Some(line)) = lines.next_line().await {
            seen.lock().unwrap().push_str(&format!("{line}\n"));
            let reply: &[u8] = if in_data {
                if line != "." {
                    continue;
                }
                in_data = false;
                final_reply.as_bytes()
            } else if line.starts_with("EHLO") {
                b"250-double\r\n250 8BITMIME\r\n"
            } else if line == "DATA" {
                in_data = true;
                b"354 go ahead\r\n"
            } else if line == "QUIT" {
                write.write_all(b"221 bye\r\n").await.unwrap();
                break;
            } else {
                b"250 ok\r\n"
            };
            write.write_all(reply).await.unwrap();
        }
    });
    (port, log)
}

fn settings(port: u16) -> EmailSettings {
    EmailSettings {
        to: "reader@kindle.com".into(),
        from: "me@example.test".into(),
        host: "127.0.0.1".into(),
        port: Some(port),
        security: Security::None,
        ..Default::default()
    }
}

fn epub(dir: &tempfile::TempDir, bytes: usize) -> std::path::PathBuf {
    let path = dir.path().join("Vol 1.epub");
    std::fs::write(&path, vec![b'x'; bytes]).unwrap();
    path
}

#[tokio::test]
async fn sends_the_book_to_the_kindle_address() {
    let (port, log) = smtp_double("250 queued\r\n").await;
    let dir = tempfile::tempdir().unwrap();
    let sent = send_book(&settings(port), &epub(&dir, 2048), "Vol 1")
        .await
        .unwrap();
    assert_eq!((sent.to.as_str(), sent.bytes), ("reader@kindle.com", 2048));
    let log = log.lock().unwrap().clone();
    assert!(log.contains("MAIL FROM:<me@example.test>"), "{log}");
    assert!(log.contains("RCPT TO:<reader@kindle.com>"), "{log}");
    assert!(log.contains("filename=\"Vol 1.epub\""), "{log}");
}

#[tokio::test]
async fn test_email_goes_to_the_sender() {
    let (port, log) = smtp_double("250 queued\r\n").await;
    let to = send_test(&settings(port)).await.unwrap();
    assert_eq!(to, "me@example.test");
    let log = log.lock().unwrap().clone();
    assert!(log.contains("RCPT TO:<me@example.test>"), "{log}");
    assert!(log.contains("Subject: mangas test email"), "{log}");
}

#[tokio::test]
async fn server_rejection_is_explained() {
    let (port, _) = smtp_double("552 5.3.4 message size exceeds limit\r\n").await;
    let dir = tempfile::tempdir().unwrap();
    let err = send_book(&settings(port), &epub(&dir, 10), "Vol 1")
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("too large"), "{err}");
}

#[tokio::test]
async fn oversized_file_is_refused_before_connecting() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.epub");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(50_000_001).unwrap();
    // Port 9 (discard) is never contacted: the size check runs first.
    let err = send_book(&settings(9), &path, "Big")
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("up to 50 MB"), "{err}");
}

#[tokio::test]
async fn unreachable_server_names_the_address() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let err = send_test(&settings(port)).await.unwrap_err().to_string();
    assert!(err.contains(&format!("127.0.0.1:{port}")), "{err}");
}

#[tokio::test]
async fn a_server_that_never_answers_times_out() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (_sock, _) = listener.accept().await.unwrap();
        std::future::pending::<()>().await;
    });
    let mut smtp = parse_sender(&settings(port)).unwrap();
    smtp.timeout = std::time::Duration::from_millis(300);
    let started = std::time::Instant::now();
    let err = send_message(&smtp, test_message(&smtp).unwrap())
        .await
        .unwrap_err()
        .to_string();
    assert_eq!(
        err,
        format!("The mail server at 127.0.0.1:{port} didn't answer in time")
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}
