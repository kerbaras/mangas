//! With no group picked, the release already on disk is the one downloaded: never a second copy.

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::download::queue::{self, pick_releases};
use mangas_core::library::{mark_downloaded, open_in_memory, upsert_chapters, upsert_manga};

#[test]
fn the_release_on_disk_is_picked_and_not_downloaded_again() {
    let lib = open_in_memory().unwrap();
    let manga = Manga {
        id: MangaId("opm".into()),
        source: "mangadex".into(),
        title: "OPM".into(),
        ..Default::default()
    };
    upsert_manga(&lib, &manga).unwrap();
    let release = |id: &str, group: &str| Chapter {
        id: ChapterId(id.into()),
        manga_id: manga.id.clone(),
        language: "en".into(),
        number: "158".into(),
        group: Some(group.into()),
        ..Default::default()
    };
    let listed = vec![
        release("r-158", "r/OnePunchMan"),
        release("koi-158", "The Koi pond."),
    ];
    upsert_chapters(&lib, "mangadex", &manga.id, "en", &listed).unwrap();
    let dir = tempfile::tempdir().unwrap();
    // What a finished one-page download leaves: the page and the page-count marker.
    std::fs::write(dir.path().join("0001.jpg"), b"page").unwrap();
    std::fs::write(dir.path().join(".mangas-pages"), "1\n").unwrap();
    mark_downloaded(&lib, "mangadex", &ChapterId("koi-158".into()), dir.path()).unwrap();

    // The source lists both releases again, with no local path, and prefers the other group.
    let preferred = ["r/OnePunchMan".to_string()];
    let picked = pick_releases(&lib, "mangadex", listed, None, &preferred).unwrap();
    let ids: Vec<&str> = picked.iter().map(|c| c.id.0.as_str()).collect();
    assert_eq!(ids, ["koi-158"]);
    let queued = queue::enqueue(&lib, "mangadex", picked, false).unwrap();
    assert!(queued.jobs.is_empty(), "no second download");
    assert_eq!(queued.on_disk.len(), 1);
}
