//! A tiny HTTP proxy for tests: absolute-form GETs and `CONNECT` tunnels, with its own host table
//! (one name only it can resolve). It records each request line it gets.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use tokio::io::{copy_bidirectional, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// The name only the proxy resolves (to loopback); `.test` names never resolve in public DNS.
pub const ONLY_PROXY: &str = "only-proxy.test";

pub struct Proxy {
    pub port: u16,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Proxy {
    pub async fn start() -> Self {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        tokio::spawn(async move {
            while let Ok((conn, _)) = listener.accept().await {
                tokio::spawn(serve(conn, log.clone()));
            }
        });
        Self { port, seen }
    }

    pub fn seen(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }
}

async fn serve(mut client: TcpStream, seen: Arc<Mutex<Vec<String>>>) {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if client.read(&mut byte).await.unwrap_or(0) == 0 {
            return;
        }
        head.push(byte[0]);
    }
    let text = String::from_utf8_lossy(&head).to_string();
    let line = text.lines().next().unwrap_or_default().to_string();
    seen.lock().unwrap().push(line.clone());
    let mut parts = line.split(' ');
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let authority = match method {
        "CONNECT" => target,
        _ => target
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or(""),
    };
    let upstream = match resolve(authority) {
        Some(addr) => TcpStream::connect(addr).await.ok(),
        None => None,
    };
    let Some(mut server) = upstream else {
        let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
        return;
    };
    let ready = match method {
        "CONNECT" => client.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await,
        _ => server.write_all(&head).await,
    };
    if ready.is_ok() {
        let _ = copy_bidirectional(&mut client, &mut server).await;
    }
}

/// The proxy's host table: [`ONLY_PROXY`] is loopback, nothing else resolves.
fn resolve(authority: &str) -> Option<SocketAddr> {
    let (host, port) = authority.rsplit_once(':')?;
    let port = port.parse().ok()?;
    (host == ONLY_PROXY).then_some(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
}
