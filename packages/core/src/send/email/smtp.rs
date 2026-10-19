//! The SMTP client: the one place that opens a connection.

use std::time::{Duration, Instant};

use lettre::transport::smtp::authentication::{Credentials, DEFAULT_MECHANISMS};
use lettre::transport::smtp::client::{AsyncSmtpConnection, TlsParameters};
use lettre::transport::smtp::extension::ClientId;
use lettre::Message;
use tokio::time::timeout;

use super::errors::{explain, Failure};
use super::{Security, Smtp};
use crate::domain::{missing_root_certificates, Error, Result};

/// Slowest upload we wait for.
const MIN_UPLOAD_BYTES_PER_SEC: u64 = 100_000;
/// The most the upload allowance adds, whatever the size: a 50 MB book needs ~170 kB/s.
pub const MAX_UPLOAD_ALLOWANCE: Duration = Duration::from_secs(300);
/// How long a delivered message waits for the server to say goodbye.
const QUIT_WAIT: Duration = Duration::from_secs(5);

/// How long a send may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deadlines {
    /// Connect, greeting, STARTTLS and login: a wrong port fails here whatever the size.
    pub handshake: Duration,
    /// The whole send: the timeout plus the upload at the slowest rate we accept, capped.
    pub total: Duration,
}

/// The deadlines for a message of `bytes` with the client `timeout`.
pub fn deadlines(timeout: Duration, bytes: usize) -> Deadlines {
    let upload = Duration::from_secs(bytes as u64 / MIN_UPLOAD_BYTES_PER_SEC);
    Deadlines {
        handshake: timeout,
        total: timeout + upload.min(MAX_UPLOAD_ALLOWANCE),
    }
}

/// Sends `message` through the server in `smtp`. lettre only times out the TCP connect, so a
/// server that accepts and never answers would hang forever: the handshake has its own
/// deadline, and the message goes out within what is left of the whole-send one
/// ([`deadlines`]). Running out drops the connection and says which step stalled.
pub async fn send(smtp: &Smtp, message: Message) -> Result<()> {
    let raw = message.formatted();
    let limits = deadlines(smtp.timeout, raw.len());
    let started = Instant::now();
    let reason = |e: lettre::transport::smtp::Error| (failure(&e), e.to_string());
    let outcome = async {
        let mut conn = timeout(limits.handshake, open(smtp, limits.handshake))
            .await
            .map_err(|_| (Failure::Timeout, String::new()))?
            .map_err(reason)?;
        let left = limits.total.saturating_sub(started.elapsed());
        timeout(left, conn.send(message.envelope(), &raw))
            .await
            .map_err(|_| (Failure::Stalled(limits.total.as_secs()), String::new()))?
            .map_err(reason)?;
        // Delivered: a server that never answers QUIT doesn't make it a failure.
        let _ = timeout(QUIT_WAIT, conn.abort()).await;
        Ok(())
    };
    outcome
        .await
        .map_err(|(f, raw): (Failure, String)| Error::Send(explain(f, &raw, &smtp.host, smtp.port)))
}

/// Connects, reads the greeting, upgrades to TLS and logs in, as lettre's transport does.
async fn open(
    smtp: &Smtp,
    connect: Duration,
) -> std::result::Result<AsyncSmtpConnection, lettre::transport::smtp::Error> {
    let hello = ClientId::default();
    let tls = || TlsParameters::new(smtp.host.clone());
    let wrapper = match smtp.security {
        Security::Tls => Some(tls()?),
        Security::Starttls | Security::None => None,
    };
    let server = (smtp.host.as_str(), smtp.port);
    let mut conn =
        AsyncSmtpConnection::connect_tokio1(server, Some(connect), &hello, wrapper, None).await?;
    if smtp.security == Security::Starttls {
        conn.starttls(tls()?, &hello).await?;
    }
    if let Some((user, password)) = &smtp.credentials {
        let credentials = Credentials::new(user.clone(), password.clone());
        conn.auth(DEFAULT_MECHANISMS, &credentials).await?;
    }
    Ok(conn)
}

fn failure(e: &lettre::transport::smtp::Error) -> Failure {
    if let Some(code) = e.status() {
        return Failure::Reply(u16::from(code));
    }
    if missing_root_certificates(e) {
        Failure::NoRootCertificates
    } else if e.is_timeout() {
        Failure::Timeout
    } else if e.is_tls() {
        Failure::Tls
    } else {
        Failure::Connection
    }
}
