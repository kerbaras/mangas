use async_trait::async_trait;

use super::fetcher::PageFetcher;
use crate::domain::Result;
use crate::sources::Source;

/// Picks a second URL for a page whose first fetch failed (see [`Source::mirror_url`]).
pub type MirrorFor<'a> = dyn Fn(&str) -> Option<String> + Send + Sync + 'a;

/// Wraps a fetcher so a failed page is tried once more on the source's mirror.
pub struct WithMirror<'a, F> {
    inner: F,
    mirror_for: Box<MirrorFor<'a>>,
}

impl<'a, F: PageFetcher> WithMirror<'a, F> {
    pub fn new(inner: F, mirror_for: Box<MirrorFor<'a>>) -> Self {
        Self { inner, mirror_for }
    }

    /// `inner` falling back to `source.mirror_url`.
    pub fn for_source(inner: F, source: &'a dyn Source) -> Self {
        Self::new(inner, Box::new(move |url| source.mirror_url(url)))
    }
}

#[async_trait]
impl<F: PageFetcher> PageFetcher for WithMirror<'_, F> {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        match self.inner.fetch(url).await {
            Ok(bytes) => Ok(bytes),
            Err(e) => match (self.mirror_for)(url).filter(|m| m != url) {
                Some(mirror) => {
                    log::warn!("page {url} failed ({e}); trying {mirror}");
                    self.inner.fetch(&mirror).await
                }
                None => Err(e),
            },
        }
    }
}
