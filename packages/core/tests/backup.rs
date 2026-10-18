use std::collections::BTreeMap;

use mangas_core::backup::{export_to, import_from, parse, VERSION};
use mangas_core::domain::{Manga, MangaId};
use mangas_core::library::{
    get_settings, list_manga, open_in_memory, set_settings, upsert_manga, ProviderSession, Settings,
};

const KNOWN: &[&str] = &["mangadex", "tapas"];

fn manga(source: &str, id: &str, title: &str) -> Manga {
    Manga {
        id: MangaId(id.into()),
        source: source.into(),
        title: title.into(),
        ..Default::default()
    }
}

fn session() -> BTreeMap<String, ProviderSession> {
    [(
        "comick".to_string(),
        ProviderSession {
            cookies: "cf=secret".into(),
            ..Default::default()
        },
    )]
    .into()
}

#[test]
fn export_then_import_restores_titles_and_preferences_without_secrets() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("library.json");
    let lib = open_in_memory().unwrap();
    let mut alpha = manga("mangadex", "a", "Alpha");
    alpha.cover_url = Some("https://example.org/a.jpg".into());
    alpha.authors = vec!["Someone".into()];
    upsert_manga(&lib, &alpha).unwrap();
    upsert_manga(&lib, &manga("tapas", "t", "Tapas one")).unwrap();
    upsert_manga(&lib, &manga("local", "l", "Scanned")).unwrap();
    set_settings(
        &lib,
        &Settings {
            data_dir: Some("/mnt/old".into()),
            default_device: Some("kindle-scribe".into()),
            provider_sessions: session(),
            ..Default::default()
        },
    )
    .unwrap();

    let summary = export_to(&lib, &file, "9.9.9").unwrap();
    assert_eq!((summary.titles, summary.skipped_local), (2, 1));
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        !text.contains("cf=secret") && !text.contains("/mnt/old"),
        "{text}"
    );
    assert!(text.contains("\"ProviderID\": \"mangadex\""));

    let fresh = open_in_memory().unwrap();
    set_settings(
        &fresh,
        &Settings {
            data_dir: Some("/mnt/new".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let report = import_from(&fresh, &file, KNOWN).unwrap();
    assert_eq!(
        (report.added, report.already, report.settings),
        (2, 0, true)
    );
    let restored = list_manga(&fresh).unwrap();
    assert!(restored.contains(&alpha), "{restored:?}");
    let settings = get_settings(&fresh).unwrap();
    assert_eq!(settings.default_device.as_deref(), Some("kindle-scribe"));
    assert_eq!(settings.data_dir.as_deref(), Some("/mnt/new"));

    let again = import_from(&fresh, &file, KNOWN).unwrap();
    assert_eq!((again.added, again.already), (0, 2));
}

#[test]
fn imports_a_haruneko_bookmark_export() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("haruneko.bookmarks");
    let entry = |provider: &str, id: &str, title: &str| {
        format!(
            r#"{{"Created":1700000000000,"Updated":1700000000000,"Title":"{title}","Media":{{"ProviderID":"{provider}","EntryID":"{id}"}},"Info":{{"ProviderID":null,"EntryID":null}}}}"#
        )
    };
    let list = [
        entry("mangadex", "uuid-1", "One"),
        entry("mangadex", "uuid-1", "One"),
        entry("somesite", "x", "Elsewhere"),
    ];
    std::fs::write(&file, format!("[{}]", list.join(","))).unwrap();
    let lib = open_in_memory().unwrap();
    let report = import_from(&lib, &file, KNOWN).unwrap();
    assert_eq!(
        (report.added, report.already, report.settings),
        (1, 1, false)
    );
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.skipped[0].title, "Elsewhere");
    assert_eq!(
        list_manga(&lib).unwrap(),
        vec![manga("mangadex", "uuid-1", "One")]
    );
}

#[test]
fn rejects_files_that_arent_backups_or_are_too_new() {
    assert!(parse("not json").is_err());
    assert!(parse(r#"{"hello": 1}"#).is_err());
    let newer = format!(
        r#"{{"format":"mangas-backup","version":{},"exported_at":"","app_version":"","bookmarks":[]}}"#,
        VERSION + 1
    );
    let err = parse(&newer).unwrap_err();
    assert!(err.to_string().contains("newer version"), "{err}");
}
