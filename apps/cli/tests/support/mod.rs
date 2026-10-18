//! A one-session SMTP double for the CLI tests.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

/// One SMTP session on a thread; returns the port and a handle yielding what the client sent.
pub fn smtp_double() -> (u16, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let (sock, _) = listener.accept().unwrap();
        let mut out = sock.try_clone().unwrap();
        let mut log = String::new();
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
                l if l.starts_with("EHLO") => b"250 double\r\n",
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
        log
    });
    (port, handle)
}
