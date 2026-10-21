//! Decoding an API reply: the protobuf `Response` is either a success or an error popup
//! ("Account Banned" without a `SESSION-TOKEN`, "Not Found" for a removed title).

use mangas_core::domain::{Error, Result};
use prost::Message;

use super::proto::{Response, SuccessResult};
use super::SOURCE_NAME;

/// The success part of `body`, or the error the API explained.
pub fn decode(body: &[u8], what: &str) -> Result<SuccessResult> {
    let response = Response::decode(body).map_err(|e| error(format!("{what}: {e}")))?;
    if let Some(success) = response.success {
        return Ok(success);
    }
    let popup = response.error.and_then(|e| e.english_popup);
    Err(match popup {
        Some(p) if p.subject.eq_ignore_ascii_case("not found") => {
            Error::NotFound(format!("{SOURCE_NAME}: {what}"))
        }
        Some(p) => error(format!("{what}: {}", explained(&p.subject, &p.body))),
        None => error(format!("{what}: empty reply")),
    })
}

/// The error popup of `body` when the API answered with one instead of a result.
pub fn rejection(body: &[u8]) -> Option<String> {
    let response = Response::decode(body).ok()?;
    if response.success.is_some() {
        return None;
    }
    let popup = response.error?.english_popup?;
    Some(explained(&popup.subject, &popup.body))
}

pub fn error(message: String) -> Error {
    Error::Source {
        name: SOURCE_NAME.to_string(),
        message,
    }
}

/// "subject (first line of the body)", or whichever of the two is there.
fn explained(subject: &str, body: &str) -> String {
    let first = body.lines().next().unwrap_or_default().trim();
    match (subject.trim(), first) {
        ("", "") => "error".to_string(),
        (s, "") => s.to_string(),
        ("", b) => b.to_string(),
        (s, b) => format!("{s} ({b})"),
    }
}
