use std::sync::Mutex;

use async_trait::async_trait;
use mangas_core::domain::{Availability, Chapter, ChapterId, Error, Manga, MangaId};
use mangas_core::library::{
    get_manga, list_chapters, open_in_memory, refresh_from_source, upsert_manga, Refreshed,
};
use mangas_core::sources::{PageUrls, Source};

fn id() -> MangaId {
    MangaId("tog".into())
}

fn chapter(number: &str) -> Chapter {
    Chapter {
        id: ChapterId(format!("ch-{number}")),
        manga_id: id(),
        title: None,
        language: "en".into(),
        volume: None,
        number: number.into(),
        local_path: None,
        group: None,
        availability: Availability::default(),
    }
}

/// A title as stored before the metadata columns existed: no credits, year or tags.
fn bare() -> Manga {
    Manga {
        id: id(),
        source: "fake".into(),
        title: "Tower of God".into(),
        ..Default::default()
    }
}

fn credited() -> Manga {
    Manga {
        authors: vec!["SIU".into()],
        year: Some(2010),
        tags: vec!["Fantasy".into()],
        ..bare()
    }
}

/// Serves a fixed record and chapter list, counting record requests.
struct FakeSource {
    manga_calls: Mutex<usize>,
}

#[async_trait]
impl Source for FakeSource {
    fn name(&self) -> &'static str {
        "fake"
    }
    async fn search(&self, _: &str, _: usize) -> Result<Vec<Manga>, Error> {
        Ok(vec![])
    }
    async fn manga(&self, _: &MangaId) -> Result<Manga, Error> {
        *self.manga_calls.lock().unwrap() += 1;
        Ok(credited())
    }
    async fn chapters(&self, _: &MangaId, _: &str) -> Result<Vec<Chapter>, Error> {
        Ok(vec![chapter("1"), chapter("2")])
    }
    async fn pages(&self, _: &ChapterId) -> Result<PageUrls, Error> {
        Ok(PageUrls(vec![]))
    }
}

fn source() -> FakeSource {
    FakeSource {
        manga_calls: Mutex::new(0),
    }
}

#[tokio::test]
async fn refreshing_a_followed_title_fills_metadata_it_was_followed_without() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &bare()).unwrap();

    let result = refresh_from_source(&lib, &source(), &id(), "en")
        .await
        .unwrap();

    assert_eq!(result, Refreshed::Saved);
    assert_eq!(get_manga(&lib, "fake", &id()).unwrap(), Some(credited()));
    let stored = list_chapters(&lib, "fake", &id(), Some("en")).unwrap();
    assert_eq!(stored.len(), 2);
}

#[tokio::test]
async fn refreshing_an_unfollowed_title_stores_nothing() {
    let lib = open_in_memory().unwrap();
    let src = source();

    let result = refresh_from_source(&lib, &src, &id(), "en").await.unwrap();

    assert_eq!(
        result,
        Refreshed::NotFollowed(vec![chapter("1"), chapter("2")])
    );
    assert_eq!(get_manga(&lib, "fake", &id()).unwrap(), None);
    assert_eq!(*src.manga_calls.lock().unwrap(), 0);
}
