//! Binary replies for APIs that answer protobuf (MANGA Plus).

use mangas_core::domain::Result;
use reqwest::Method;

use super::super::redact_query;
use super::hop::Outgoing;
use super::SiteClient;

impl SiteClient {
    /// GET returning the bytes of a 2xx response; other statuses become errors, as in
    /// [`Self::get`].
    pub async fn get_bytes(&self, path: &str, query: &[(&str, String)]) -> Result<Vec<u8>> {
        let url = self.url(path);
        log::debug!("{}: GET {url} {}", self.config.info.id, redact_query(query));
        let request = Outgoing {
            binary: true,
            ..Outgoing::new(Method::GET, self.parse(&url, query)?, None)
        };
        let mut reply = self
            .send(path, request, self.config.retry, self.config.timeout)
            .await?;
        let raw = std::mem::take(&mut reply.raw);
        self.ok_body(path, reply)?;
        Ok(raw)
    }
}
