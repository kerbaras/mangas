//! SMTP settings as stored, and their checked form.

use std::time::Duration;

use lettre::message::Mailbox;
use serde::{Deserialize, Serialize};

use crate::domain::{Error, Result};

/// How the connection to the SMTP server is secured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Security {
    /// Plain connection upgraded with STARTTLS (port 587); the usual choice.
    #[default]
    Starttls,
    /// TLS from the first byte (port 465).
    Tls,
    /// No encryption: only for a relay on the local machine or network.
    None,
}

impl Security {
    pub fn default_port(self) -> u16 {
        match self {
            Security::Starttls => 587,
            Security::Tls => 465,
            Security::None => 25,
        }
    }
}

/// Send to Kindle email settings. Empty strings mean "not set".
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EmailSettings {
    /// The device's Send to Kindle address (`name@kindle.com`).
    pub to: String,
    /// Sender address; Amazon only accepts mail from its approved list. Empty: `username`.
    pub from: String,
    pub host: String,
    /// `None`: the usual port for `security`.
    pub port: Option<u16>,
    pub security: Security,
    pub username: String,
    /// Empty in the stored settings: the password lives in the system keychain (see
    /// [`crate::send::password`]), or here only on a computer without one. Never part of a
    /// backup.
    pub password: String,
}

/// How long the server may stay silent: connect, greeting and each command. The upload gets
/// extra time on top, see `smtp::send`.
pub const SMTP_TIMEOUT: Duration = Duration::from_secs(60);

/// Settings checked and parsed, ready for the SMTP client.
#[derive(Debug, Clone, PartialEq)]
pub struct Smtp {
    pub host: String,
    pub port: u16,
    pub security: Security,
    /// User name and password, when a user name is set.
    pub credentials: Option<(String, String)>,
    pub from: Mailbox,
    pub to: Mailbox,
    /// [`SMTP_TIMEOUT`] unless a caller (a test) needs it shorter.
    pub timeout: Duration,
}

/// Checks the settings needed to email a book to the Kindle.
pub fn parse(settings: &EmailSettings) -> Result<Smtp> {
    parse_with(settings, true)
}

/// Like [`parse`] without the Kindle address: the message goes to the sender (test email).
pub fn parse_sender(settings: &EmailSettings) -> Result<Smtp> {
    parse_with(settings, false)
}

fn parse_with(s: &EmailSettings, to_kindle: bool) -> Result<Smtp> {
    let host = s.host.trim();
    let from = if s.from.trim().is_empty() {
        s.username.trim()
    } else {
        s.from.trim()
    };
    let missing: Vec<&str> = [
        (to_kindle && s.to.trim().is_empty(), "Kindle address"),
        (from.is_empty(), "sender address"),
        (host.is_empty(), "SMTP server"),
    ]
    .into_iter()
    .filter_map(|(gone, name)| gone.then_some(name))
    .collect();
    if !missing.is_empty() {
        return Err(Error::Send(format!(
            "Email delivery isn't set up: missing {}",
            missing.join(", ")
        )));
    }
    let from = mailbox(from, "sender address")?;
    let to = if to_kindle {
        mailbox(s.to.trim(), "Kindle address")?
    } else {
        from.clone()
    };
    let user = s.username.trim();
    if !user.is_empty() && s.security == Security::None && !is_loopback(host) {
        return Err(Error::Send(format!(
            "Signing in to {host} without TLS would send the password in clear text; choose \
             STARTTLS or TLS (the provider's settings say which)"
        )));
    }
    Ok(Smtp {
        host: host.to_string(),
        port: s.port.unwrap_or(s.security.default_port()),
        security: s.security,
        credentials: (!user.is_empty()).then(|| (user.to_string(), s.password.clone())),
        from,
        to,
        timeout: SMTP_TIMEOUT,
    })
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

fn mailbox(address: &str, what: &str) -> Result<Mailbox> {
    address
        .parse()
        .map_err(|_| Error::Send(format!("The {what} '{address}' is not an email address")))
}
