//! The SMTP client: the one place that opens a connection.

use std::time::Duration;

use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use super::errors::{explain, Failure};
use super::{Security, Smtp};
use crate::domain::{Error, Result};

/// Slowest upload we wait for; a 50 MB book gets about eight minutes on top of the timeout.
const MIN_UPLOAD_BYTES_PER_SEC: u64 = 100_000;

/// Sends `message` through the server in `smtp`. lettre only times out the TCP connect, so a
/// server that accepts and never answers (a wrong port) would hang forever; the whole exchange
/// runs under one deadline: `smtp.timeout` plus the upload time at the slowest rate we accept.
pub async fn send(smtp: &Smtp, message: Message) -> Result<()> {
    let raw = message.formatted();
    let upload = Duration::from_secs(raw.len() as u64 / MIN_UPLOAD_BYTES_PER_SEC);
    let transport = transport(smtp)?;
    let exchange = transport.send_raw(message.envelope(), &raw);
    let error = |failure, raw: &str| Error::Send(explain(failure, raw, &smtp.host, smtp.port));
    match tokio::time::timeout(smtp.timeout + upload, exchange).await {
        Err(_) => Err(error(Failure::Timeout, "")),
        Ok(sent) => sent
            .map(|_| ())
            .map_err(|e| error(failure(&e), &e.to_string())),
    }
}

fn transport(smtp: &Smtp) -> Result<AsyncSmtpTransport<Tokio1Executor>> {
    let tls = |host: &str| {
        TlsParameters::new(host.to_string())
            .map_err(|e| Error::Send(format!("TLS setup for {host}: {e}")))
    };
    let tls = match smtp.security {
        Security::Starttls => Tls::Required(tls(&smtp.host)?),
        Security::Tls => Tls::Wrapper(tls(&smtp.host)?),
        Security::None => Tls::None,
    };
    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp.host)
        .port(smtp.port)
        .tls(tls)
        .timeout(Some(smtp.timeout));
    if let Some((user, password)) = &smtp.credentials {
        builder = builder.credentials(Credentials::new(user.clone(), password.clone()));
    }
    Ok(builder.build())
}

fn failure(e: &lettre::transport::smtp::Error) -> Failure {
    if let Some(code) = e.status() {
        return Failure::Reply(u16::from(code));
    }
    if e.is_timeout() {
        Failure::Timeout
    } else if e.is_tls() {
        Failure::Tls
    } else {
        Failure::Connection
    }
}
