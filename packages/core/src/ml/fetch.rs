//! Streams a download into memory with progress and cooperative cancellation.

use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::StreamExt;

use crate::domain::{Error, Result};

/// Message of the error a cancelled download returns.
pub const CANCELLED: &str = "download cancelled";

pub fn is_cancelled(err: &Error) -> bool {
    matches!(err, Error::Other(m) if m == CANCELLED)
}

/// Downloads `url`; `on_progress` gets (bytes so far, total if known) after every chunk, and the
/// download stops with [`CANCELLED`] as soon as `cancel` is set.
pub async fn fetch(
    url: &str,
    on_progress: &mut (dyn FnMut(u64, Option<u64>) + Send),
    cancel: &AtomicBool,
) -> Result<Vec<u8>> {
    let resp = reqwest::get(url).await?.error_for_status()?;
    let total = resp.content_length();
    let mut stream = resp.bytes_stream();
    let mut bytes: Vec<u8> = Vec::with_capacity(total.unwrap_or(0) as usize);
    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Other(CANCELLED.into()));
        }
        bytes.extend_from_slice(&chunk?);
        on_progress(bytes.len() as u64, total);
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(Error::Other(CANCELLED.into()));
    }
    Ok(bytes)
}
