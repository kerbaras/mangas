//! What the webview sees of the settings: everything but the credentials. The SMTP password only
//! shows as `kindle_email.has_password`, and a provider session only as its `saved_at`. A password
//! goes in through [`set_password`] and never comes back; sessions stay with the backend.

use rusqlite::OptionalExtension;
use serde_json::Value;

use super::resolve::{locked, reads_keychain, test_settings};
use super::settings::{load_settings, save_settings, Saved};
use super::store::{SecretStore, StoreError};
use crate::domain::{Error, Result};
use crate::library::{get_settings, Library, Settings};
use crate::send::EmailSettings;

/// Whether the keychain holds a password, remembered so a settings read never has to ask it
/// (a locked keychain would prompt on every window focus).
const SAVED_KEY: &str = "smtp_password_saved";

/// `settings` without `kindle_email.password`, with `kindle_email.has_password` instead, and each
/// provider session cut down to `{ saved_at }` (no cookies or user agent); every other key is
/// kept as stored.
pub fn redact(settings: &Settings, has_password: bool) -> Result<Value> {
    let mut value = serde_json::to_value(settings).map_err(|e| Error::Other(e.to_string()))?;
    if let Some(email) = value.get_mut("kindle_email").and_then(Value::as_object_mut) {
        email.remove("password");
        email.insert("has_password".into(), Value::Bool(has_password));
    }
    if let Some(sessions) = value
        .get_mut("provider_sessions")
        .and_then(Value::as_object_mut)
    {
        for session in sessions.values_mut() {
            let saved_at = session.get("saved_at").cloned().unwrap_or(Value::Null);
            *session = serde_json::json!({ "saved_at": saved_at });
        }
    }
    Ok(value)
}

/// [`redact`] with whether a password is saved, without reading the keychain.
pub fn public_settings(lib: &Library, settings: &Settings) -> Result<Value> {
    redact(settings, has_password(lib, settings)?)
}

/// A password in the settings file (no keychain), or one the keychain last said it holds. The
/// first read without the flag (settings from before it) records the rule sends already follow:
/// a user name with no password in the file means the password went to the keychain.
pub fn has_password(lib: &Library, settings: &Settings) -> Result<bool> {
    if !settings.kindle_email.password.is_empty() {
        return Ok(true);
    }
    let sql = "SELECT value FROM setting WHERE key = ?1";
    let saved: Option<String> = lib
        .conn()?
        .query_row(sql, [SAVED_KEY], |r| r.get(0))
        .optional()?;
    Ok(match saved.as_deref() {
        Some(flag) => flag == "1",
        None => {
            let saved = reads_keychain(&settings.kindle_email);
            remember(lib, saved)?;
            saved
        }
    })
}

/// Records whether the keychain holds a password, for [`has_password`].
pub(super) fn remember(lib: &Library, saved: bool) -> Result<()> {
    lib.conn()?.execute(
        "INSERT INTO setting (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        [SAVED_KEY, if saved { "1" } else { "0" }],
    )?;
    Ok(())
}

/// Saves `password` (empty: clears it) on the same path as the CLI's `settings email`: the
/// keychain, or the settings file without one. A locked keychain fails and changes nothing.
pub fn set_password(lib: &Library, store: &dyn SecretStore, password: &str) -> Result<Saved> {
    store.retry();
    let before = load_settings(lib, store)?;
    if password.is_empty() && before.kindle_email.password.is_empty() {
        // Nothing readable to compare with (a locked keychain reads as empty): clear it anyway.
        if let Err(StoreError::Locked(why)) = store.delete() {
            return Err(locked(&why));
        }
        remember(lib, false)?;
        return Ok(Saved {
            settings: before,
            fallback: None,
        });
    }
    let mut edited = before.clone();
    edited.kindle_email.password = password.into();
    save_settings(lib, store, Some(&before), &edited)
}

/// Saves a settings edit from the webview, which never carries the password: whatever is in
/// `kindle_email.password` is ignored and the saved password stays.
pub fn save_preferences(
    lib: &Library,
    store: &dyn SecretStore,
    base: Option<&Settings>,
    edited: &Settings,
) -> Result<Saved> {
    let blank = |s: &Settings| {
        let mut s = s.clone();
        s.kindle_email.password.clear();
        s
    };
    let mut edited = blank(edited);
    let base = match base {
        Some(base) => blank(base),
        // A full replacement still keeps the saved password: replace over what is stored now,
        // which also keeps the stored provider sessions like `edit_settings` without a base.
        None => {
            let current = blank(&get_settings(lib)?);
            edited.provider_sessions = current.provider_sessions.clone();
            current
        }
    };
    save_settings(lib, store, Some(&base), &edited)
}

/// The settings a test email from the webview uses: the form's `draft`, which never carries the
/// saved password, with the one still in the settings file (no keychain) unless the user
/// `typed` one; then like [`test_settings`].
pub fn draft_test_settings(
    lib: &Library,
    store: &dyn SecretStore,
    draft: &EmailSettings,
    typed: bool,
) -> Result<EmailSettings> {
    if typed {
        return test_settings(store, draft, true);
    }
    let draft = EmailSettings {
        password: get_settings(lib)?.kindle_email.password,
        ..draft.clone()
    };
    test_settings(store, &draft, false)
}
