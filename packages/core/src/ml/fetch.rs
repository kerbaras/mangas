//! Streams a download into memory with progress and cooperative cancellation.

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

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
    let resp = or_cancel(reqwest::get(url), cancel)
        .await??
        .error_for_status()?;
    let total = resp.content_length();
    let mut stream = resp.bytes_stream();
    let mut bytes: Vec<u8> = Vec::with_capacity(total.unwrap_or(0) as usize);
    while let Some(chunk) = or_cancel(stream.next(), cancel).await? {
        bytes.extend_from_slice(&chunk?);
        on_progress(bytes.len() as u64, total);
    }
    Ok(bytes)
}

/// Awaits `fut`, giving up with [`CANCELLED`] once `cancel` is set, even if the server stalls.
async fn or_cancel<T>(fut: impl Future<Output = T>, cancel: &AtomicBool) -> Result<T> {
    let watch = async {
        while !cancel.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    tokio::select! {
        biased;
        () = watch => Err(Error::Other(CANCELLED.into())),
        out = fut => Ok(out),
    }
}
