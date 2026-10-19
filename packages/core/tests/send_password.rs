//! The SMTP password in a secret store instead of the settings file (in-memory store).

use mangas_core::library::{self, Library, Settings};
use mangas_core::send::password::{
    email_settings, load_settings, save_settings, storage, with_password, MemoryStore, Storage,
};
use mangas_core::send::EmailSettings;

fn email(password: &str) -> EmailSettings {
    EmailSettings {
        host: "smtp.example.test".into(),
        username: "me@example.test".into(),
        password: password.into(),
        ..Default::default()
    }
}

fn with_email(password: &str) -> Settings {
    Settings {
        kindle_email: email(password),
        ..Default::default()
    }
}

fn stored(lib: &Library) -> String {
    library::get_settings(lib).unwrap().kindle_email.password
}

#[test]
fn a_plain_text_password_moves_to_the_keychain_on_first_load() {
    let lib = library::open_in_memory().unwrap();
    library::set_settings(&lib, &with_email("old-secret")).unwrap();
    let store = MemoryStore::default();
    let loaded = load_settings(&lib, &store).unwrap();
    assert_eq!(loaded.kindle_email.password, "old-secret");
    assert_eq!(store.peek().as_deref(), Some("old-secret"));
    assert_eq!(stored(&lib), "");
    assert_eq!(
        library::get_settings(&lib).unwrap().kindle_email.host,
        "smtp.example.test"
    );
    assert_eq!(load_settings(&lib, &store).unwrap(), loaded);
    assert_eq!(storage(&lib, &store).unwrap(), Storage::Keychain);
}

#[test]
fn without_a_keychain_the_password_stays_in_the_file() {
    let lib = library::open_in_memory().unwrap();
    library::set_settings(&lib, &with_email("old-secret")).unwrap();
    let store = MemoryStore::unavailable("no secret service");
    let loaded = load_settings(&lib, &store).unwrap();
    assert_eq!(loaded.kindle_email.password, "old-secret");
    assert_eq!(stored(&lib), "old-secret");
    assert_eq!(storage(&lib, &store).unwrap(), Storage::File);
    let send = email_settings(&lib, &store).unwrap();
    assert_eq!(send.password, "old-secret");
}

#[test]
fn a_new_password_is_saved_to_the_keychain_and_blank_in_the_file() {
    let lib = library::open_in_memory().unwrap();
    let store = MemoryStore::default();
    let base = load_settings(&lib, &store).unwrap();
    let saved = save_settings(&lib, &store, Some(&base), &with_email("new-secret")).unwrap();
    assert_eq!(saved.fallback, None);
    assert_eq!(saved.settings.kindle_email.password, "new-secret");
    assert_eq!(saved.settings.kindle_email.host, "smtp.example.test");
    assert_eq!(store.peek().as_deref(), Some("new-secret"));
    assert_eq!(stored(&lib), "");
    assert_eq!(email_settings(&lib, &store).unwrap().password, "new-secret");
}

#[test]
fn a_new_password_replaces_one_left_in_the_file() {
    let lib = library::open_in_memory().unwrap();
    library::set_settings(&lib, &with_email("stale")).unwrap();
    let store = MemoryStore::default();
    let base = Settings::default();
    save_settings(&lib, &store, Some(&base), &with_email("fresh")).unwrap();
    assert_eq!(stored(&lib), "");
    assert_eq!(
        load_settings(&lib, &store).unwrap().kindle_email.password,
        "fresh"
    );
}

#[test]
fn clearing_the_password_removes_it_from_the_keychain() {
    let lib = library::open_in_memory().unwrap();
    let store = MemoryStore::with("secret");
    let base = load_settings(&lib, &store).unwrap();
    let saved = save_settings(&lib, &store, Some(&base), &with_email("")).unwrap();
    assert_eq!(store.peek(), None);
    assert_eq!(saved.settings.kindle_email.password, "");
}

#[test]
fn an_edit_that_keeps_the_password_does_not_touch_the_keychain() {
    let lib = library::open_in_memory().unwrap();
    let base = Settings {
        kindle_email: email("secret"),
        ..Default::default()
    };
    let edited = Settings {
        update_check_hours: Some(12),
        ..base.clone()
    };
    // A failing store would turn any keychain call into a fallback.
    let untouchable = MemoryStore::unavailable("must not be called");
    let saved = save_settings(&lib, &untouchable, Some(&base), &edited).unwrap();
    assert_eq!(saved.fallback, None);
    assert_eq!(saved.settings.kindle_email.password, "secret");
    assert_eq!(saved.settings.update_check_hours, Some(12));
    assert_eq!(stored(&lib), "");
}

#[test]
fn saving_without_a_keychain_writes_the_file_and_says_why() {
    let lib = library::open_in_memory().unwrap();
    let store = MemoryStore::unavailable("no secret service");
    let saved = save_settings(&lib, &store, None, &with_email("secret")).unwrap();
    assert_eq!(saved.fallback.as_deref(), Some("no secret service"));
    assert_eq!(stored(&lib), "secret");
}

#[test]
fn a_carried_password_wins_and_a_missing_one_comes_from_the_keychain() {
    let store = MemoryStore::with("kept");
    assert_eq!(
        with_password(&store, &email("typed")).unwrap().password,
        "typed"
    );
    assert_eq!(with_password(&store, &email("")).unwrap().password, "kept");
    let anonymous = EmailSettings {
        username: String::new(),
        ..email("")
    };
    let broken = MemoryStore::unavailable("locked");
    assert_eq!(with_password(&broken, &anonymous).unwrap().password, "");
    // A blank-password account still reaches the server, as it did before the keychain.
    assert_eq!(with_password(&broken, &email("")).unwrap().password, "");
}

#[test]
fn a_locked_keychain_fails_the_send_instead_of_sending_without_a_password() {
    let locked = MemoryStore::locked("prompt dismissed");
    let err = with_password(&locked, &email("")).unwrap_err().to_string();
    assert!(err.starts_with("The system keychain is locked"), "{err}");
    assert!(err.ends_with("(prompt dismissed)"), "{err}");
    assert_eq!(
        with_password(&locked, &email("typed")).unwrap().password,
        "typed"
    );
}

#[test]
fn a_locked_keychain_is_told_apart_from_no_keychain() {
    let lib = library::open_in_memory().unwrap();
    library::set_settings(&lib, &with_email("")).unwrap();
    let locked = MemoryStore::locked("collection is locked");
    assert_eq!(storage(&lib, &locked).unwrap(), Storage::Locked);
    let missing = MemoryStore::unavailable("no secret service");
    assert_eq!(storage(&lib, &missing).unwrap(), Storage::File);
    assert_eq!(
        storage(&lib, &MemoryStore::default()).unwrap(),
        Storage::Keychain
    );
}
