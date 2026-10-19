use mangas_core::library::{self, Settings};
use mangas_core::send::{EmailSettings, Security};
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
        default_format: Some(mangas_core::export::Format::Pdf),
        provider_sessions: Default::default(),
        preferred_languages: vec!["es-la".into(), "en".into()],
        provider_options: [(
            "mangadex".to_string(),
            serde_json::json!({ "data_saver": true }),
        )]
        .into(),
        providers_enabled: [("mangadot".to_string(), false)].into(),
        provider_languages: [("mangadex".to_string(), vec!["es".to_string()])].into(),
        update_check_hours: Some(0),
        update_notifications: Some(false),
        app_update_channel: Some(mangas_core::app_update::Channel::Rc),
        app_update_checks: Some(false),
        kindle_email: EmailSettings {
            to: "reader@kindle.com".into(),
            host: "smtp.example.test".into(),
            port: Some(2525),
            security: Security::Tls,
            ..Default::default()
        },
        theme: Some(library::Theme::Dark),
        download_concurrency: Some(6),
        proxy_mode: Some(library::ProxyMode::Custom),
        proxy_url: Some("http://127.0.0.1:8888".into()),
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

#[test]
fn languages_default_to_english_and_are_normalised() {
    assert_eq!(Settings::default().languages(), [library::DEFAULT_LANGUAGE]);
    let s = Settings {
        preferred_languages: vec![" PT-BR ".into(), "".into(), "en".into()],
        ..Settings::default()
    };
    assert_eq!(s.languages(), ["pt-br", "en"]);
}

#[test]
fn providers_default_to_enabled_with_the_global_languages() {
    let s = Settings {
        preferred_languages: vec!["fr".into()],
        providers_enabled: [("mangadot".to_string(), false)].into(),
        provider_languages: [
            ("mangadex".to_string(), vec![" ES-LA ".into()]),
            ("comick".to_string(), vec![" ".into()]),
        ]
        .into(),
        ..Settings::default()
    };
    assert!(!s.provider_enabled("mangadot"));
    assert!(s.provider_enabled("mangadex"));
    assert!(s.provider_enabled("added-later"));
    assert_eq!(s.languages_for("mangadex"), ["es-la"]);
    assert_eq!(s.languages_for("comick"), ["fr"]);
    assert_eq!(s.languages_for("weebcentral"), ["fr"]);
    assert_eq!(Settings::default().languages_for("mangadex"), ["en"]);
}

#[test]
fn settings_stored_before_provider_management_enable_every_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mangas.db");
    library::open(&path).unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "INSERT INTO setting (key, value) VALUES ('app', ?1)",
            [r#"{"preferred_languages":["en"],"provider_sessions":{}}"#],
        )
        .unwrap();
    let got = library::get_settings(&library::open(&path).unwrap()).unwrap();
    assert!(got.providers_enabled.is_empty());
    assert!(got.provider_enabled("mangadex"));
    assert_eq!(got.languages_for("mangadex"), ["en"]);
}

#[test]
fn update_checks_default_to_every_six_hours_with_notifications() {
    let defaults = Settings::default();
    assert_eq!(defaults.update_hours(), library::DEFAULT_UPDATE_HOURS);
    assert_eq!(defaults.update_hours(), 6);
    assert!(defaults.notify_updates());
    let off = Settings {
        update_check_hours: Some(0),
        update_notifications: Some(false),
        app_update_channel: Some(mangas_core::app_update::Channel::Rc),
        app_update_checks: Some(false),
        ..Settings::default()
    };
    assert_eq!((off.update_hours(), off.notify_updates()), (0, false));
}
