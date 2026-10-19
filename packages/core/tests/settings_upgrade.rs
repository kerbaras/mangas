//! Settings from before `smtp_password_saved` (main kept the password in the keychain and no
//! flag): the first read says whether one is saved without asking the keychain.

use mangas_core::library::{self, Library, Settings};
use mangas_core::send::password::{has_password, public_settings, storage, MemoryStore, Storage};
use mangas_core::send::EmailSettings;
use serde_json::Value;

/// A library with the settings row main leaves: the account, an empty password, no flag.
fn main_shaped() -> (tempfile::TempDir, Library) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mangas.db");
    library::open(&path).unwrap();
    let settings = Settings {
        kindle_email: EmailSettings {
            host: "smtp.example.test".into(),
            username: "me@example.test".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let row = serde_json::to_string(&settings).unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute("INSERT INTO setting (key, value) VALUES ('app', ?1)", [row])
        .unwrap();
    let lib = library::open(&path).unwrap();
    (dir, lib)
}

#[test]
fn a_password_saved_before_the_upgrade_shows_on_the_first_read() {
    let (_dir, lib) = main_shaped();
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
fn without_a_keychain_the_first_check_corrects_an_account_with_no_password() {
    let (_dir, lib) = main_shaped();
    let settings = library::get_settings(&lib).unwrap();
    assert!(has_password(&lib, &settings).unwrap());
    let none = MemoryStore::unavailable("no secret service");
    assert_eq!(storage(&lib, &none).unwrap(), Storage::File);
    assert!(!has_password(&lib, &settings).unwrap());
}

#[test]
fn no_account_before_the_upgrade_means_no_password() {
    let lib = library::open_in_memory().unwrap();
    let settings = library::get_settings(&lib).unwrap();
    let dto = public_settings(&lib, &settings).unwrap();
    assert_eq!(dto["kindle_email"]["has_password"], Value::Bool(false));
}
