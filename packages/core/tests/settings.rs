use mangas_core::library::{self, Settings};
use mangas_core::smartcut::Preset;

#[test]
fn settings_default_to_empty() {
    let lib = library::open_in_memory().unwrap();
    assert_eq!(library::get_settings(&lib).unwrap(), Settings::default());
}

#[test]
fn settings_round_trip_and_overwrite() {
    let lib = library::open_in_memory().unwrap();
    let first = Settings {
        data_dir: Some("/tmp/mangas".into()),
        default_device: Some("kindle-scribe".into()),
        default_preset: Some(Preset::FewestCuts),
        provider_sessions: Default::default(),
    };
    library::set_settings(&lib, &first).unwrap();
    assert_eq!(library::get_settings(&lib).unwrap(), first);

    let second = Settings {
        data_dir: None,
        ..first
    };
    library::set_settings(&lib, &second).unwrap();
    assert_eq!(library::get_settings(&lib).unwrap(), second);
}

#[test]
fn settings_tolerate_unknown_and_missing_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mangas.db");
    library::open(&path).unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "INSERT INTO setting (key, value) VALUES ('app', ?1)",
            [r#"{"default_device":"kindle-oasis3","future_field":1}"#],
        )
        .unwrap();

    let got = library::get_settings(&library::open(&path).unwrap()).unwrap();
    assert_eq!(got.default_device.as_deref(), Some("kindle-oasis3"));
    assert_eq!(got.data_dir, None);
    assert_eq!(got.default_preset, None);
}
