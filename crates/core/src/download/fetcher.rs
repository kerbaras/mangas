use async_trait::async_trait;
use reqwest::Client;

use crate::domain::Result;

/// Fetches one page image. Abstracted so downloads can be tested without a network.
#[async_trait]
pub trait PageFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>>;
}

pub struct HttpFetcher {
    http: Client,
}

impl HttpFetcher {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent(concat!("mangas/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { http })
    }
}

#[async_trait]
impl PageFetcher for HttpFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.http.get(url).send().await?.error_for_status()?;
        Ok(response.bytes().await?.to_vec())
    }
}
