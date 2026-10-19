//! The settings as the webview gets them: no SMTP password, only whether one is saved.

use mangas_core::library::{self, Settings};
use mangas_core::send::password::{
    draft_test_settings, email_settings, has_password, public_settings, redact, save_preferences,
    set_password, storage, MemoryStore, Storage,
};
use mangas_core::send::EmailSettings;
use serde_json::Value;

fn with_email(password: &str) -> Settings {
    Settings {
        kindle_email: EmailSettings {
            host: "smtp.example.test".into(),
            username: "me@example.test".into(),
            password: password.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn keys(v: &Value) -> Vec<String> {
    let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    keys
}

#[test]
fn the_dto_has_every_settings_key_but_no_password() {
    let settings = with_email("hunter2");
    let full = serde_json::to_value(&settings).unwrap();
    let dto = redact(&settings, true).unwrap();
    assert_eq!(keys(&dto), keys(&full), "every settings key is kept");
    let email = &dto["kindle_email"];
    assert!(email.get("password").is_none());
    assert_eq!(email["has_password"], Value::Bool(true));
    assert_eq!(email["host"], "smtp.example.test");
    let mut expected = keys(&full["kindle_email"]);
    expected.retain(|k| k != "password");
    expected.push("has_password".into());
    expected.sort();
    assert_eq!(keys(email), expected);
    assert!(!dto.to_string().contains("hunter2"));
}

#[test]
fn provider_sessions_show_only_when_they_were_saved() {
    let mut settings = with_email("");
    settings.provider_sessions.insert(
        "comick".into(),
        library::ProviderSession {
            cookies: "cf_clearance=secret-cookie".into(),
            user_agent: "Secret UA/1.0".into(),
            saved_at: Some("2026-10-19T12:00:00Z".into()),
        },
    );
    let full = serde_json::to_value(&settings).unwrap();
    let dto = redact(&settings, false).unwrap();
    assert_eq!(keys(&dto), keys(&full), "every settings key is kept");
    assert_eq!(
        dto["provider_sessions"],
        serde_json::json!({ "comick": { "saved_at": "2026-10-19T12:00:00Z" } })
    );
    let text = dto.to_string();
    assert!(
        !text.contains("secret-cookie") && !text.contains("Secret UA"),
        "{text}"
    );
}

#[test]
fn a_password_set_through_the_keychain_never_shows_in_the_dto() {
    let lib = library::open_in_memory().unwrap();
    // A fresh install: the app reads the settings before an account is entered.
    let fresh = library::get_settings(&lib).unwrap();
    assert!(!has_password(&lib, &fresh).unwrap());
    library::set_settings(&lib, &with_email("")).unwrap();
    let store = MemoryStore::default();
    let settings = library::get_settings(&lib).unwrap();
    assert!(!has_password(&lib, &settings).unwrap());
    let saved = set_password(&lib, &store, "s3cret-pw").unwrap();
    assert_eq!(saved.fallback, None);
    assert_eq!(store.peek().as_deref(), Some("s3cret-pw"));
    let settings = library::get_settings(&lib).unwrap();
    assert_eq!(settings.kindle_email.password, "");
    let dto = public_settings(&lib, &settings).unwrap();
    assert_eq!(dto["kindle_email"]["has_password"], Value::Bool(true));
    assert!(!dto.to_string().contains("s3cret-pw"));
    assert_eq!(email_settings(&lib, &store).unwrap().password, "s3cret-pw");

    set_password(&lib, &store, "").unwrap();
    assert_eq!(store.peek(), None);
    let settings = library::get_settings(&lib).unwrap();
    assert!(!has_password(&lib, &settings).unwrap());
}

#[test]
fn without_a_keychain_the_password_goes_to_the_file_and_still_stays_out_of_the_dto() {
    let lib = library::open_in_memory().unwrap();
    let store = MemoryStore::unavailable("no secret service");
    let saved = set_password(&lib, &store, "file-pw").unwrap();
    assert!(saved.fallback.is_some());
    let settings = library::get_settings(&lib).unwrap();
    assert_eq!(settings.kindle_email.password, "file-pw");
    assert_eq!(storage(&lib, &store).unwrap(), Storage::File);
    let dto = public_settings(&lib, &settings).unwrap();
    assert_eq!(dto["kindle_email"]["has_password"], Value::Bool(true));
    assert!(!dto.to_string().contains("file-pw"));
}

#[test]
fn a_locked_keychain_fails_the_password_save_and_keeps_the_old_one() {
    let lib = library::open_in_memory().unwrap();
    let store = MemoryStore::locked("prompt dismissed");
    let err = set_password(&lib, &store, "new").unwrap_err();
    assert!(err.to_string().contains("locked"), "{err}");
    assert!(set_password(&lib, &store, "").is_err());
    assert_eq!(
        library::get_settings(&lib).unwrap().kindle_email.password,
        ""
    );
}

#[test]
fn a_settings_edit_from_the_webview_never_changes_the_password() {
    let lib = library::open_in_memory().unwrap();
    let store = MemoryStore::default();
    set_password(&lib, &store, "kept").unwrap();
    let base = library::get_settings(&lib).unwrap();
    let mut edited = with_email("injected");
    edited.kindle_email.host = "smtp.other.test".into();
    let saved = save_preferences(&lib, &store, Some(&base), &edited).unwrap();
    assert_eq!(saved.settings.kindle_email.host, "smtp.other.test");
    assert_eq!(store.peek().as_deref(), Some("kept"));
    assert_eq!(
        library::get_settings(&lib).unwrap().kindle_email.password,
        ""
    );
    let settings = library::get_settings(&lib).unwrap();
    assert!(has_password(&lib, &settings).unwrap());
}

#[test]
fn a_password_saved_before_the_upgrade_shows_on_the_first_read() {
    // What main left: the account in the settings row, the password in the keychain, no flag.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mangas.db");
    library::open(&path).unwrap();
    let row = serde_json::to_string(&with_email("")).unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute("INSERT INTO setting (key, value) VALUES ('app', ?1)", [row])
        .unwrap();
    let lib = library::open(&path).unwrap();
    let settings = library::get_settings(&lib).unwrap();
    let dto = public_settings(&lib, &settings).unwrap();
    assert_eq!(dto["kindle_email"]["has_password"], Value::Bool(true));
    let again = public_settings(&lib, &settings).unwrap();
    assert_eq!(again["kindle_email"]["has_password"], Value::Bool(true));
    // Once the keychain is asked (the Kindle pane), its answer wins.
    assert_eq!(
        storage(&lib, &MemoryStore::default()).unwrap(),
        Storage::Keychain
    );
    assert!(!has_password(&lib, &settings).unwrap());
}

#[test]
fn no_account_before_the_upgrade_means_no_password() {
    let lib = library::open_in_memory().unwrap();
    let settings = library::get_settings(&lib).unwrap();
    let dto = public_settings(&lib, &settings).unwrap();
    assert_eq!(dto["kindle_email"]["has_password"], Value::Bool(false));
}

#[test]
fn a_settings_replacement_without_a_base_keeps_the_saved_password() {
    for store in [
        MemoryStore::unavailable("no secret service"),
        MemoryStore::default(),
    ] {
        let lib = library::open_in_memory().unwrap();
        set_password(&lib, &store, "kept").unwrap();
        let mut edited = with_email("");
        edited.kindle_email.host = "smtp.other.test".into();
        save_preferences(&lib, &store, None, &edited).unwrap();
        let sent = email_settings(&lib, &store).unwrap();
        assert_eq!(sent.host, "smtp.other.test");
        assert_eq!(sent.password, "kept");
    }
}

#[test]
fn a_test_email_uses_the_saved_password_unless_one_is_typed() {
    let draft = with_email("").kindle_email;
    for store in [
        MemoryStore::unavailable("no secret service"),
        MemoryStore::default(),
    ] {
        let lib = library::open_in_memory().unwrap();
        set_password(&lib, &store, "saved").unwrap();
        let untyped = draft_test_settings(&lib, &store, &draft, false).unwrap();
        assert_eq!(untyped.password, "saved");
        let typed = EmailSettings {
            password: "typed".into(),
            ..draft.clone()
        };
        let tested = draft_test_settings(&lib, &store, &typed, true).unwrap();
        assert_eq!(tested.password, "typed");
    }
}
