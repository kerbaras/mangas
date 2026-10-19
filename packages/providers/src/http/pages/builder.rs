//! Optional knobs on a [`PageClient`]: retries, a body cap, and the test-only host rules.

#[cfg(any(test, feature = "test-support"))]
use std::net::IpAddr;

use mangas_core::domain::Result;

use super::{build_http, PageClient};
use crate::http::RetryPolicy;

impl PageClient<'_> {
    pub fn with_retry(self, retry: RetryPolicy) -> Self {
        Self { retry, ..self }
    }

    /// Refuse bodies over `max_bytes` (announced or streamed) instead of reading them whole.
    pub fn with_max_bytes(self, max_bytes: usize) -> Self {
        Self {
            max_bytes: Some(max_bytes),
            ..self
        }
    }

    /// Also reaches loopback and private addresses: only for tests against a mock server.
    pub fn allow_private_hosts(self) -> Result<Self> {
        Ok(Self {
            http: build_http(false, &self.network)?,
            public_only: false,
            ..self
        })
    }

    /// Resolves every host name to `ip`, still refusing non-public ones: only for tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn resolving_to(self, ip: IpAddr) -> Result<Self> {
        Ok(Self {
            http: super::redirect::build_http_resolving(ip)?,
            public_only: true,
            ..self
        })
    }
}
