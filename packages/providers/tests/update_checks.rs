//! Update checks against a local MangaDex: one title failing doesn't lose the others' news.

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::updates::{summary, UpdatePolicy};
use mangas_core::library::{
    list_entries, open_in_memory, set_chapter_group, set_reading_status, upsert_chapters,
    upsert_manga, Library, ReadingStatus,
};
use mangas_providers::updates::check_updates_at;
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const EMINENCE: &str = "91d928a1-7237-4d03-abc3-4b1440cc7d55";
const BROKEN: &str = "00000000-0000-4000-8000-000000000000";
const DROPPED: &str = "11111111-1111-4111-8111-111111111111";

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/mangadex/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn follow(lib: &Library, id: &str, title: &str) -> MangaId {
    let manga = Manga {
        id: MangaId(id.into()),
        source: "mangadex".into(),
        title: title.into(),
        ..Default::default()
    };
    upsert_manga(lib, &manga).unwrap();
    // A first English sync, so English chapters found by the check count as new.
    let old = Chapter {
        id: ChapterId(format!("{id}-old")),
        manga_id: manga.id.clone(),
        language: "en".into(),
        number: "0".into(),
        ..Default::default()
    };
    upsert_chapters(lib, "mangadex", &manga.id, "en", &[old]).unwrap();
    manga.id
}

async fn server() -> MockServer {
    let server = MockServer::start().await;
    let feed = format!("/manga/{EMINENCE}/feed");
    for (offset, file) in [
        ("0", "feed_groups_page1.json"),
        ("6", "feed_groups_page2.json"),
    ] {
        Mock::given(path(feed.as_str()))
            .and(query_param("offset", offset))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture(file)))
            .mount(&server)
            .await;
    }
    Mock::given(path(format!("/manga/{BROKEN}/feed")))
        .respond_with(ResponseTemplate::new(404).set_body_string("{\"result\":\"error\"}"))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn a_failing_title_is_reported_and_the_others_still_count() {
    let lib = open_in_memory().unwrap();
    let eminence = follow(&lib, EMINENCE, "Eminence in Shadow");
    follow(&lib, BROKEN, "Broken");
    let dropped = follow(&lib, DROPPED, "Dropped");
    set_reading_status(&lib, "mangadex", &dropped, ReadingStatus::Dropped).unwrap();
    let server = server().await;

    let report = check_updates_at(&lib, &UpdatePolicy::default(), &server.uri())
        .await
        .unwrap();
    assert_eq!((report.checked, report.skipped), (2, 1));
    assert_eq!(report.updates.len(), 1, "{report:?}");
    let update = &report.updates[0];
    assert_eq!(update.manga_id, eminence);
    assert!(!update.chapters.is_empty());
    assert_eq!(report.failures.len(), 1, "{report:?}");
    assert_eq!(report.failures[0].manga_id.0, BROKEN);
    assert!(!report.failures[0].challenge);
    assert_eq!(summary(&report).unreachable, 1);
    let requests = server.received_requests().await.unwrap_or_default();
    assert!(
        requests.iter().all(|r| !r.url.path().contains(DROPPED)),
        "dropped titles are not fetched"
    );

    // Stored and badged; a second run finds nothing new.
    let entries = list_entries(&lib).unwrap();
    let badge = entries.iter().find(|e| e.manga.id == eminence).unwrap();
    assert_eq!(badge.chapters.new, summary(&report).chapters);
    let again = check_updates_at(&lib, &UpdatePolicy::default(), &server.uri())
        .await
        .unwrap();
    assert!(again.updates.is_empty(), "{again:?}");
}

#[tokio::test]
async fn a_title_narrowed_to_one_group_only_counts_that_groups_chapters() {
    let lib = open_in_memory().unwrap();
    let eminence = follow(&lib, EMINENCE, "Eminence in Shadow");
    set_chapter_group(&lib, "mangadex", &eminence, Some("Mitsugoshi")).unwrap();
    let server = server().await;

    let report = check_updates_at(&lib, &UpdatePolicy::default(), &server.uri())
        .await
        .unwrap();
    let chapters = &report.updates[0].chapters;
    assert!(!chapters.is_empty(), "{report:?}");
    assert!(
        chapters
            .iter()
            .all(|c| c.group.as_deref() == Some("Mitsugoshi")),
        "{chapters:?}"
    );
    let entries = list_entries(&lib).unwrap();
    assert_eq!(entries[0].chapters.new, summary(&report).chapters);
}
