//! The image step's request: what a download sends, restricted to public hosts.

use mangas_core::download::is_public_http_url;
use mangas_core::sources::Header;
use reqwest::header::RANGE;
use reqwest::redirect::{Action, Attempt};
use reqwest::Client;

use crate::http::header_map;

/// One-byte ranged GET of `url`: hosts that ignore `Range` answer 200 with the body.
pub(super) async fn first_byte(
    client: Client,
    url: String,
    headers: Vec<Header>,
) -> Result<u16, String> {
    if !is_public_http_url(&url) {
        return Err(format!("not a public http(s) url: {url}"));
    }
    let headers = header_map(&headers).map_err(|e| e.to_string())?;
    let request = client.get(&url).headers(headers).header(RANGE, "bytes=0-0");
    let response = request.send().await.map_err(|e| e.to_string())?;
    Ok(response.status().as_u16())
}

pub(super) fn public_redirects_only(attempt: Attempt) -> Action {
    if attempt.previous().len() >= 10 {
        attempt.error("too many redirects")
    } else if is_public_http_url(attempt.url().as_str()) {
        attempt.follow()
    } else {
        attempt.error("redirect to a non-public url")
    }
}
