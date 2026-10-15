use reqwest::Client;

use super::mapping::{parse_error_response, SOURCE_NAME};
use super::rate_limit::TokenBucket;
use crate::domain::Result;

pub const API_BASE: &str = "https://api.mangadex.org";
const USER_AGENT: &str = concat!(
    "mangas/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/kerbaras/mangas)"
);

/// Rate-limited HTTP client for the MangaDex API (5 requests per second).
pub struct ApiClient {
    http: Client,
    base: String,
    bucket: TokenBucket,
}

impl ApiClient {
    pub fn new(base: impl Into<String>) -> Result<Self> {
        let http = Client::builder().user_agent(USER_AGENT).build()?;
        Ok(Self {
            http,
            base: base.into(),
            bucket: TokenBucket::new(5, 5),
        })
    }

    /// GET `path` with `query` and return the body text; non-2xx responses become
    /// [`Error::Source`] carrying the API's own error detail.
    pub async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<String> {
        self.bucket.acquire().await;
        let url = format!("{}{}", self.base, path);
        log::debug!("{SOURCE_NAME}: GET {url} {query:?}");
        let response = self.http.get(&url).query(query).send().await?;
        let status = response.status();
        let body = response.text().await?;
        if status.is_success() {
            Ok(body)
        } else {
            Err(parse_error_response(status.as_u16(), &body))
        }
    }
}
