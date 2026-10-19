//! Shared CLI test fixtures: a local SMTP double and noisy chapter pages.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

/// One SMTP session on a thread; returns the port and a handle yielding what the client sent.
#[allow(dead_code)]
pub fn smtp_double() -> (u16, std::thread::JoinHandle<String>) {
    smtp_sessions(1)
}

/// `sessions` SMTP sessions in a row (one per email), taking any login; yields every line sent.
pub fn smtp_sessions(sessions: usize) -> (u16, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let mut log = String::new();
        for _ in 0..sessions {
            let (sock, _) = listener.accept().unwrap();
            session(sock, &mut log);
        }
        log
    });
    (port, handle)
}

/// A chapter of noisy pages, so their JPEGs are as big as real scans (about 400 kB each).
#[allow(dead_code)]
pub fn noisy_chapter(dir: &std::path::Path, pages: usize) {
    std::fs::create_dir_all(dir).unwrap();
    let mut seed = 11u32;
    for i in 0..pages {
        let img = image::RgbImage::from_fn(800, 1200, |_, _| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let v = (seed >> 24) as u8;
            image::Rgb([v, v, v])
        });
        img.save(dir.join(format!("{i:03}.png"))).unwrap();
    }
}

fn session(sock: std::net::TcpStream, log: &mut String) {
    let mut out = sock.try_clone().unwrap();
    out.write_all(b"220 double ESMTP\r\n").unwrap();
    let mut in_data = false;
    for line in BufReader::new(sock).lines() {
        let line = line.unwrap();
        log.push_str(&line);
        log.push('\n');
        let reply: &[u8] = match line.as_str() {
            "." if in_data => {
                in_data = false;
                b"250 queued\r\n"
            }
            _ if in_data => continue,
            l if l.starts_with("EHLO") => b"250-double\r\n250 AUTH PLAIN LOGIN\r\n",
            l if l.starts_with("AUTH") => b"235 ok\r\n",
            "DATA" => {
                in_data = true;
                b"354 go ahead\r\n"
            }
            "QUIT" => {
                out.write_all(b"221 bye\r\n").unwrap();
                break;
            }
            _ => b"250 ok\r\n",
        };
        out.write_all(reply).unwrap();
    }
}
