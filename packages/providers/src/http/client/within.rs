//! GETs with their own timeout instead of the client's, for the few calls a big title can make
//! slow (MangaDot's whole chapter list, #164).

use std::time::Duration;

use mangas_core::domain::Result;
use reqwest::Method;

use super::{redact_query, Outgoing, SiteClient};

impl SiteClient {
    /// [`Self::get`] with `timeout` per attempt in place of the client's.
    pub async fn get_within(
        &self,
        path: &str,
        query: &[(&str, String)],
        timeout: Duration,
    ) -> Result<String> {
        let url = self.url(path);
        let id = self.config.info.id;
        log::debug!("{id}: GET {url} {} within {timeout:?}", redact_query(query));
        let request = Outgoing::new(Method::GET, self.parse(&url, query)?, None);
        let reply = self.send(path, request, self.config.retry, timeout).await?;
        self.ok_body(path, reply)
    }
}
