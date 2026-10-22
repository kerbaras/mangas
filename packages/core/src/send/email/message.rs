//! The messages sent: a book with its EPUB attached, or a test note. Pure builders.

use lettre::message::header::ContentType;
use lettre::message::{Attachment, MultiPart, SinglePart};
use lettre::Message;

use super::Smtp;
use crate::domain::{Error, Result};

/// Largest document Amazon's Send to Kindle accepts by email (50 MB).
pub const MAX_ATTACHMENT_BYTES: u64 = 50_000_000;

/// The attachment type of a file Amazon converts from email: EPUB or PDF.
fn content_type(file_name: &str) -> Result<ContentType> {
    let ext = file_name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase());
    let kind = match ext.as_deref() {
        Some("epub") => "application/epub+zip",
        Some("pdf") => "application/pdf",
        _ => {
            return Err(Error::Send(format!(
                "Send to Kindle by email takes EPUB or PDF files, not {file_name}. Export it as \
                 EPUB, or copy it to the Kindle over USB."
            )))
        }
    };
    ContentType::parse(kind).map_err(|e| Error::Send(e.to_string()))
}

/// Whether Send to Kindle by email takes `file_name` (an EPUB or a PDF).
pub fn emailable(file_name: &str) -> bool {
    content_type(file_name).is_ok()
}

/// Refuses a file Amazon would bounce, before anything is uploaded.
pub fn check_size(bytes: u64) -> Result<()> {
    if bytes <= MAX_ATTACHMENT_BYTES {
        return Ok(());
    }
    Err(Error::Send(format!(
        "The book is {:.1} MB; Send to Kindle by email takes up to {} MB. Export fewer chapters \
         per book, or copy it to the Kindle over USB.",
        bytes as f64 / 1e6,
        MAX_ATTACHMENT_BYTES / 1_000_000
    )))
}

/// The book `title` as an EPUB or PDF attachment named `file_name`.
pub fn book_message(smtp: &Smtp, title: &str, file_name: &str, book: Vec<u8>) -> Result<Message> {
    let kind = content_type(file_name)?;
    let attachment = Attachment::new(file_name.to_string()).body(book, kind);
    Message::builder()
        .from(smtp.from.clone())
        .to(smtp.to.clone())
        .subject(title)
        .multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::plain(format!("{title}, sent by mangas.")))
                .singlepart(attachment),
        )
        .map_err(build_error)
}

/// A short note to the sender's own address, to check the settings.
pub fn test_message(smtp: &Smtp) -> Result<Message> {
    Message::builder()
        .from(smtp.from.clone())
        .to(smtp.to.clone())
        .subject("mangas test email")
        .body(format!(
            "mangas reached {}:{} and sent this message. Books will go to your Kindle \
             address from {}; make sure Amazon lists it as an approved sender.",
            smtp.host, smtp.port, smtp.from.email
        ))
        .map_err(build_error)
}

fn build_error(e: lettre::error::Error) -> Error {
    Error::Send(format!("Couldn't build the email: {e}"))
}
