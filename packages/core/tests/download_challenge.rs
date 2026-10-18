//! A page behind a browser verification fails the chapter with `Error::Challenge`, so the app
//! can recognise it instead of showing a plain incomplete-chapter error.

use async_trait::async_trait;
use mangas_core::domain::{ChapterId, Error};
use mangas_core::download::{download_pages, PageFetcher};
use mangas_core::sources::PageUrls;

struct ChallengeOn(&'static str);

#[async_trait]
impl PageFetcher for ChallengeOn {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, Error> {
        if url.ends_with(self.0) {
            return Err(Error::Challenge {
                name: "site".into(),
                site: "https://cdn.example".into(),
            });
        }
        Err(Error::Other(format!("boom {url}")))
    }
}

#[tokio::test]
async fn a_challenged_page_fails_the_chapter_as_a_challenge() {
    let dir = tempfile::tempdir().unwrap();
    let urls = PageUrls(
        ["1.png", "2.png", "3.png"]
            .iter()
            .map(|p| format!("https://cdn.example/{p}"))
            .collect(),
    );
    let chapter = ChapterId("c".into());
    let err = download_pages(&ChallengeOn("2.png"), &chapter, &urls, dir.path(), &|_| {})
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Challenge { .. }), "{err}");
    assert!(err.to_string().starts_with("challenge:"), "{err}");
}
