//! Book creators looked up from the library by the exported chapters' folders.

use std::path::Path;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::library::{
    export_creators, mark_downloaded, open_in_memory, upsert_chapters, upsert_manga, Library,
};

fn follow(lib: &Library, id: &str, authors: &[&str], artists: &[&str]) {
    let manga = Manga {
        id: MangaId(id.into()),
        source: "mangadex".into(),
        title: id.into(),
        authors: authors.iter().map(|a| a.to_string()).collect(),
        artists: artists.iter().map(|a| a.to_string()).collect(),
        ..Default::default()
    };
    upsert_manga(lib, &manga).unwrap();
    let chapter = Chapter {
        id: ChapterId(format!("{id}1")),
        manga_id: manga.id.clone(),
        language: "en".into(),
        number: "1".into(),
        ..Default::default()
    };
    upsert_chapters(
        lib,
        "mangadex",
        &manga.id,
        "en",
        std::slice::from_ref(&chapter),
    )
    .unwrap();
    let dir = format!("/lib/{id}/1");
    mark_downloaded(lib, "mangadex", &chapter.id, Path::new(&dir)).unwrap();
}

fn creators(lib: &Library, dirs: &[&str]) -> Vec<String> {
    let spelled: Vec<Vec<&Path>> = dirs.iter().map(|d| vec![Path::new(*d)]).collect();
    export_creators(lib, &spelled).unwrap()
}

#[test]
fn a_series_credits_its_authors_then_its_other_artists() {
    let lib = open_in_memory().unwrap();
    follow(&lib, "yotsuba", &["Azuma Kiyohiko"], &["Azuma Kiyohiko"]);
    follow(&lib, "frieren", &["Kanehito Yamada"], &["Tsukasa Abe"]);
    assert_eq!(creators(&lib, &["/lib/yotsuba/1/"]), ["Azuma Kiyohiko"]);
    assert_eq!(
        creators(&lib, &["/lib/frieren/1", "/lib/yotsuba/1"]),
        ["Kanehito Yamada", "Tsukasa Abe", "Azuma Kiyohiko"]
    );
}

#[test]
fn uncredited_or_unknown_folders_credit_nobody() {
    let lib = open_in_memory().unwrap();
    follow(&lib, "bare", &[], &[]);
    assert!(creators(&lib, &["/lib/bare/1"]).is_empty());
    assert!(creators(&lib, &["/elsewhere/1"]).is_empty());
}
