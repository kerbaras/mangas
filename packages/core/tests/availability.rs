use async_trait::async_trait;
use mangas_core::domain::{Availability, Chapter, ChapterId, Error, Manga, MangaId, MangaStatus};
use mangas_core::download::{
    download_chapter_with, ensure_available, skipped_summary, split_available, PageFetcher,
};
use mangas_core::library::{list_chapters, open, open_in_memory, upsert_chapters, upsert_manga};
use mangas_core::sources::{PageUrls, Source};

const EXTERNAL: &str = "https://mangaplus.shueisha.co.jp/viewer/1000000";

fn chapter(number: &str, availability: Availability) -> Chapter {
    Chapter {
        id: ChapterId(format!("ch-{number}")),
        manga_id: MangaId("m".into()),
        title: None,
        language: "en".into(),
        volume: None,
        number: number.into(),
        local_path: None,
        group: None,
        availability,
    }
}

fn external() -> Availability {
    Availability::External {
        url: Some(EXTERNAL.into()),
    }
}

fn manga() -> Manga {
    Manga {
        id: MangaId("m".into()),
        source: "tapas".into(),
        title: "M".into(),
        description: String::new(),
        cover_url: None,
        status: MangaStatus::Ongoing,
        ..Default::default()
    }
}

#[test]
fn availability_serialises_with_a_kind_tag_and_defaults_to_available() {
    let json = serde_json::to_value(chapter("1", external())).unwrap();
    assert_eq!(
        json["availability"],
        serde_json::json!({ "kind": "external", "url": EXTERNAL })
    );
    let mut old = json.clone();
    old.as_object_mut().unwrap().remove("availability");
    let parsed: Chapter = serde_json::from_value(old).unwrap();
    assert_eq!(parsed.availability, Availability::Available);
}

#[test]
fn columns_round_trip_and_unknown_kinds_read_as_available() {
    use Availability::{Available, Locked};
    let linkless = Availability::External { url: None };
    let read = |kind, url: Option<&str>| Availability::from_columns(kind, url.map(String::from));
    for a in [Available, Locked, external(), linkless.clone()] {
        let (kind, url) = a.to_columns();
        assert_eq!(read(kind, url), a);
    }
    assert_eq!(read("external", Some("")), linkless);
    assert_eq!(read("???", None), Available);
}

#[test]
fn library_stores_and_refreshes_availability() {
    let lib = open_in_memory().unwrap();
    upsert_manga(&lib, &manga()).unwrap();
    let id = MangaId("m".into());
    let listed = [
        chapter("1", Availability::Available),
        chapter("2", Availability::Locked),
        chapter("3", external()),
    ];
    upsert_chapters(&lib, "tapas", &id, "en", &listed).unwrap();
    assert_eq!(list_chapters(&lib, "tapas", &id, None).unwrap(), listed);

    let unlocked = [listed[0].clone(), chapter("2", Availability::Available)];
    upsert_chapters(&lib, "tapas", &id, "en", &unlocked).unwrap();
    let stored = list_chapters(&lib, "tapas", &id, None).unwrap();
    assert_eq!(stored, unlocked, "a chapter unlocked on the site follows");
}

#[test]
fn migration_marks_existing_chapters_available() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.db");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        for sql in [
            include_str!("../migrations/0001_init.sql"),
            include_str!("../migrations/0002_settings.sql"),
            include_str!("../migrations/0003_chapter_group.sql"),
        ] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 3).unwrap();
        conn.execute(
            "INSERT INTO manga (id, source, title) VALUES ('m', 'tapas', 'M')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chapter (id, manga_id, source, language, number)
             VALUES ('ch-1', 'm', 'tapas', 'en', '1')",
            [],
        )
        .unwrap();
    }
    let lib = open(&path).unwrap();
    let chapters = list_chapters(&lib, "tapas", &MangaId("m".into()), None).unwrap();
    assert_eq!(chapters.len(), 1);
    assert_eq!(chapters[0].availability, Availability::Available);
}

#[test]
fn split_keeps_order_and_downloaded_chapters() {
    let mut on_disk = chapter("4", Availability::Locked);
    on_disk.local_path = Some("/lib/4".into());
    let all = vec![
        chapter("1", Availability::Available),
        chapter("2", Availability::Locked),
        chapter("3", external()),
        on_disk,
        chapter("5", Availability::Available),
    ];
    let (ok, skipped) = split_available(all);
    let numbers = |cs: &[Chapter]| cs.iter().map(|c| c.number.clone()).collect::<Vec<_>>();
    assert_eq!(numbers(&ok), ["1", "4", "5"]);
    assert_eq!(numbers(&skipped), ["2", "3"]);
    assert_eq!(
        skipped_summary(&skipped).as_deref(),
        Some("skipped 2 chapters: 1 locked, 1 on another site")
    );
    assert_eq!(skipped_summary(&[]), None);
    assert_eq!(
        skipped_summary(&skipped[..1]).as_deref(),
        Some("skipped 1 chapter: 1 locked")
    );
}

#[test]
fn explicit_request_names_the_reason() {
    let locked = ensure_available(&chapter("7", Availability::Locked)).unwrap_err();
    assert!(matches!(locked, Error::Unavailable { .. }));
    assert_eq!(
        locked.to_string(),
        "chapter 7 can't be downloaded: it is locked on the site (paid, wait-until-free or early access)"
    );
    let ext = ensure_available(&chapter("8", external())).unwrap_err();
    assert!(ext.to_string().contains(EXTERNAL), "{ext}");
    assert!(ensure_available(&chapter("9", Availability::Available)).is_ok());
}

struct NoPages;

#[async_trait]
impl Source for NoPages {
    fn name(&self) -> &'static str {
        "tapas"
    }
    async fn search(&self, _: &str, _: usize) -> Result<Vec<Manga>, Error> {
        Ok(vec![])
    }
    async fn manga(&self, id: &MangaId) -> Result<Manga, Error> {
        Err(Error::NotFound(id.0.clone()))
    }
    async fn chapters(&self, _: &MangaId, _: &str) -> Result<Vec<Chapter>, Error> {
        Ok(vec![])
    }
    async fn pages(&self, c: &ChapterId) -> Result<PageUrls, Error> {
        panic!("pages of {c} requested for an unavailable chapter")
    }
}

#[async_trait]
impl PageFetcher for NoPages {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, Error> {
        panic!("fetched {url}")
    }
}

#[tokio::test]
async fn download_of_a_locked_chapter_fails_before_any_request() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("ch");
    let locked = chapter("2", Availability::Locked);
    let err = download_chapter_with(&NoPages, &NoPages, &locked, &dest, &|_| {})
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Unavailable { .. }), "{err}");
    assert!(!dest.exists());
}
