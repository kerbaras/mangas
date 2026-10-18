//! SMTP deadlines (#141): a silent server fails within the handshake limit whatever the
//! book's size, and only the upload gets a size-based allowance, capped.

use std::time::{Duration, Instant};

use mangas_core::send::email::{
    book_message, deadlines, explain, parse_sender, send_message, test_message, Failure,
    MAX_UPLOAD_ALLOWANCE,
};
use mangas_core::send::{EmailSettings, Security};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

fn smtp(port: u16) -> mangas_core::send::email::Smtp {
    let settings = EmailSettings {
        to: "reader@kindle.com".into(),
        from: "me@example.test".into(),
        host: "127.0.0.1".into(),
        port: Some(port),
        security: Security::None,
        ..Default::default()
    };
    let mut smtp = parse_sender(&settings).unwrap();
    smtp.timeout = Duration::from_millis(300);
    smtp
}

/// A server that answers the handshake and DATA, then goes silent once the message is in.
async fn stalls_after_data() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (sock, _) = listener.accept().await.unwrap();
        let (read, mut write) = sock.into_split();
        let mut lines = BufReader::new(read).lines();
        write.write_all(b"220 slow ESMTP\r\n").await.unwrap();
        while let Ok(Some(line)) = lines.next_line().await {
            let reply: &[u8] = match line.as_str() {
                l if l.starts_with("EHLO") => b"250-slow\r\n250 8BITMIME\r\n",
                "DATA" => b"354 go ahead\r\n",
                "." => break,
                l if l.starts_with("MAIL") || l.starts_with("RCPT") => b"250 ok\r\n",
                _ => continue,
            };
            write.write_all(reply).await.unwrap();
        }
        std::future::pending::<()>().await;
    });
    port
}

#[test]
fn only_the_upload_grows_with_the_size_and_it_is_capped() {
    let minute = Duration::from_secs(60);
    let tiny = deadlines(minute, 2_000);
    assert_eq!((tiny.handshake, tiny.total), (minute, minute));
    let five_mb = deadlines(minute, 5_000_000);
    assert_eq!(
        (five_mb.handshake, five_mb.total),
        (minute, minute * 2 - Duration::from_secs(10))
    );
    let fifty_mb = deadlines(minute, 50_000_000);
    assert_eq!(fifty_mb.handshake, minute);
    assert_eq!(fifty_mb.total, minute + MAX_UPLOAD_ALLOWANCE);
}

#[tokio::test]
async fn a_silent_server_fails_a_big_book_within_the_handshake_limit() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (_sock, _) = listener.accept().await.unwrap();
        std::future::pending::<()>().await;
    });
    let smtp = smtp(port);
    // 5 MB: the old whole-exchange deadline gave this 50 s on top of the timeout.
    let book = book_message(&smtp, "Vol 1", "Vol 1.epub", vec![b'x'; 5_000_000]).unwrap();
    let started = Instant::now();
    let err = send_message(&smtp, book).await.unwrap_err().to_string();
    assert_eq!(
        err,
        format!("The mail server at 127.0.0.1:{port} didn't answer in time")
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a_server_that_stalls_after_the_upload_is_given_up_on_with_a_clear_message() {
    let port = stalls_after_data().await;
    let smtp = smtp(port);
    let started = Instant::now();
    let err = send_message(&smtp, test_message(&smtp).unwrap())
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.starts_with(&format!(
            "The mail server at 127.0.0.1:{port} stopped answering while the book was being sent"
        )),
        "{err}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn the_stall_message_says_how_long_it_waited() {
    let msg = |secs| explain(Failure::Stalled(secs), "", "smtp.example.test", 587);
    assert!(msg(60).contains("gave up after 60 s"), "{}", msg(60));
    assert!(msg(360).contains("gave up after 6 min"), "{}", msg(360));
    assert!(msg(360).contains("copy the book over USB"), "{}", msg(360));
}
