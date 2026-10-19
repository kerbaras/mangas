//! SMTP failures in words a user can act on.

use crate::domain::NO_ROOT_CERTIFICATES;

/// What went wrong talking to the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// The server answered with this reply code.
    Reply(u16),
    /// No answer to the connection, greeting or login.
    Timeout,
    /// The server stopped answering while the message went out; gave up after this many
    /// seconds.
    Stalled(u64),
    Tls,
    /// The system has no root certificates to check the server's.
    NoRootCertificates,
    /// Couldn't connect, or the connection dropped.
    Connection,
}

/// A message for `failure` at `host:port`; `raw` is the client's own description.
pub fn explain(failure: Failure, raw: &str, host: &str, port: u16) -> String {
    let at = format!("{host}:{port}");
    match failure {
        Failure::Reply(530) => {
            format!("The mail server at {at} needs a user name and password ({raw})")
        }
        Failure::Reply(534 | 535) => format!(
            "The mail server refused the user name or password. Gmail, iCloud and Outlook need \
             an app password here ({raw})"
        ),
        Failure::Reply(552 | 523) => format!(
            "The mail server refused the message as too large. Your provider may take less than \
             Amazon's 50 MB; export fewer chapters per book or copy it over USB ({raw})"
        ),
        Failure::Reply(550 | 553 | 554) => format!(
            "The mail server refused the message; check the sender and Kindle addresses ({raw})"
        ),
        Failure::Reply(code) if (400..500).contains(&code) => {
            format!("The mail server is busy or limiting messages; try again later ({raw})")
        }
        Failure::Reply(_) => format!("The mail server refused the message ({raw})"),
        Failure::Timeout => format!("The mail server at {at} didn't answer in time"),
        Failure::Stalled(secs) => format!(
            "The mail server at {at} stopped answering while the book was being sent; gave up \
             after {}. Try again later, or copy the book over USB",
            minutes(secs)
        ),
        Failure::Tls => format!(
            "Couldn't set up a secure connection to {at}; port 465 uses TLS, 587 uses STARTTLS \
             ({raw})"
        ),
        Failure::NoRootCertificates => format!("Couldn't connect to {at}: {NO_ROOT_CERTIFICATES}"),
        Failure::Connection => format!("Couldn't reach the mail server at {at} ({raw})"),
    }
}

fn minutes(secs: u64) -> String {
    match secs {
        0..=89 => format!("{secs} s"),
        _ => format!("{} min", (secs + 30) / 60),
    }
}
