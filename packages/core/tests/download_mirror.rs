//! A page that fails on its first URL is retried once on the source's mirror.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use mangas_core::domain::{Error, Result};
use mangas_core::download::{PageFetcher, WithMirror};

#[derive(Default)]
struct Scripted {
    calls: Mutex<Vec<String>>,
}

struct Shared(Arc<Scripted>);

#[async_trait]
impl PageFetcher for Shared {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        self.0.fetch(url).await
    }
}

#[async_trait]
impl PageFetcher for Scripted {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        self.calls.lock().unwrap().push(url.to_string());
        if url.contains("broken") {
            Err(Error::Other(format!("down: {url}")))
        } else {
            Ok(url.as_bytes().to_vec())
        }
    }
}

fn mirror(url: &str) -> Option<String> {
    url.contains("node")
        .then(|| url.replace("broken-node", "mirror"))
}

#[tokio::test]
async fn a_failed_page_is_fetched_from_the_mirror() {
    let fetcher = WithMirror::new(Scripted::default(), Box::new(mirror));
    let bytes = fetcher.fetch("https://broken-node/p1").await.unwrap();
    assert_eq!(bytes, b"https://mirror/p1");
}

#[tokio::test]
async fn a_working_page_never_touches_the_mirror() {
    let scripted = Arc::new(Scripted::default());
    let fetcher = WithMirror::new(Shared(scripted.clone()), Box::new(mirror));
    fetcher.fetch("https://node/p1").await.unwrap();
    assert_eq!(*scripted.calls.lock().unwrap(), ["https://node/p1"]);
}

#[tokio::test]
async fn without_a_mirror_the_original_error_is_kept() {
    let fetcher = WithMirror::new(Scripted::default(), Box::new(|_: &str| None));
    let err = fetcher.fetch("https://broken/p1").await.unwrap_err();
    assert!(err.to_string().contains("down: https://broken/p1"), "{err}");
}
