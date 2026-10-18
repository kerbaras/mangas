//! Send to Kindle by email: SMTP settings, the message with the EPUB attached, the size guard
//! and readable SMTP errors. Only [`send_book`] and [`send_test`] touch the file system and the
//! network.

mod config;
mod errors;
mod message;
mod smtp;

use std::path::Path;

pub use config::{parse, parse_sender, EmailSettings, Security, Smtp, SMTP_TIMEOUT};
pub use errors::{explain, Failure};
pub use message::{book_message, check_size, test_message, MAX_ATTACHMENT_BYTES};
pub use smtp::send as send_message;

use crate::domain::{Error, Result};

/// What was sent: the recipient and the attachment size in bytes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Sent {
    pub to: String,
    pub bytes: u64,
}

/// Emails the EPUB at `file` to the Kindle address in `settings`. Files over
/// [`MAX_ATTACHMENT_BYTES`] are refused before connecting.
pub async fn send_book(settings: &EmailSettings, file: &Path, title: &str) -> Result<Sent> {
    let smtp = parse(settings)?;
    let bytes = tokio::fs::metadata(file).await?.len();
    check_size(bytes)?;
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| Error::Send(format!("{} is not a file", file.display())))?;
    let body = tokio::fs::read(file).await?;
    smtp::send(&smtp, book_message(&smtp, title, &name, body)?).await?;
    Ok(Sent {
        to: smtp.to.email.to_string(),
        bytes,
    })
}

/// Sends a short test message to the sender's own address, which checks the server, the
/// security mode and the credentials without putting anything on the Kindle. Returns the
/// address it went to.
pub async fn send_test(settings: &EmailSettings) -> Result<String> {
    let smtp = parse_sender(settings)?;
    smtp::send(&smtp, test_message(&smtp)?).await?;
    Ok(smtp.to.email.to_string())
}
