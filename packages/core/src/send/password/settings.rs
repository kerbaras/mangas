//! The app settings with the SMTP password kept in a [`SecretStore`] instead of the settings
//! file. Shared by the desktop app and the CLI so both find the password in the same place.

use serde::Serialize;

use super::store::{SecretStore, StoreError};
use crate::domain::Result;
use crate::library::{edit_settings, get_settings, Library, Settings};
use crate::send::EmailSettings;

/// Where the SMTP password is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Storage {
    Keychain,
    /// Unencrypted in the app settings: there is no keychain to put it in.
    File,
    /// In the keychain, as far as anyone can tell: it is locked or won't be read.
    Locked,
}

/// A saved edit: the settings as stored, password filled in, and the keychain error when the
/// new password had to go to the settings file instead.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    pub settings: Settings,
    pub fallback: Option<String>,
}

/// The settings with the password filled in from the keychain (empty when it can't be read).
/// A password still in the settings file moves to the keychain first; see [`migrate`].
pub fn load_settings(lib: &Library, store: &dyn SecretStore) -> Result<Settings> {
    let mut settings = migrate(lib, store)?;
    if settings.kindle_email.password.is_empty() {
        settings.kindle_email.password = reveal(store);
    }
    Ok(settings)
}

/// The email settings for a send, with the password.
pub fn email_settings(lib: &Library, store: &dyn SecretStore) -> Result<EmailSettings> {
    with_password(store, &migrate(lib, store)?.kindle_email)
}

/// `email` with its password: the one it carries (one still in the settings file), else the
/// keychain's. Without a user name no password is needed; a keychain that can't be read gives
/// an empty one, as before the keychain, and the server says whether it takes that.
pub fn with_password(store: &dyn SecretStore, email: &EmailSettings) -> Result<EmailSettings> {
    if !email.password.is_empty() || email.username.trim().is_empty() {
        return Ok(email.clone());
    }
    Ok(EmailSettings {
        password: reveal(store),
        ..email.clone()
    })
}

/// Where the password is (or would be) kept: in the file while one is still there, else in
/// the keychain when it answers.
pub fn storage(lib: &Library, store: &dyn SecretStore) -> Result<Storage> {
    if !get_settings(lib)?.kindle_email.password.is_empty() {
        return Ok(Storage::File);
    }
    Ok(match store.get() {
        Ok(_) => Storage::Keychain,
        Err(StoreError::Locked(_)) => Storage::Locked,
        Err(StoreError::Unavailable(_)) => Storage::File,
    })
}

/// Saves an edit like [`edit_settings`]. A changed password goes to the keychain and the
/// settings file keeps an empty one; without a keychain it is written to the file and
/// `fallback` says why. An unchanged password never touches the keychain.
pub fn save_settings(
    lib: &Library,
    store: &dyn SecretStore,
    base: Option<&Settings>,
    edited: &Settings,
) -> Result<Saved> {
    let password = edited.kindle_email.password.clone();
    let changed = base.is_none_or(|b| b.kindle_email.password != password);
    let (mut base, mut edited) = (base.cloned(), edited.clone());
    let mut fallback = None;
    // What to put back if the save fails; unknown (None) when the keychain can't be read.
    let before = changed.then(|| store.get().ok()).flatten();
    if changed {
        let kept = if password.is_empty() {
            store.delete()
        } else {
            store.set(&password)
        };
        match kept {
            Ok(()) => {
                // The merge writes the edited (empty) value only when it differs from base.
                if let Some(b) = base.as_mut().filter(|_| !password.is_empty()) {
                    b.kindle_email.password = password.clone();
                }
                edited.kindle_email.password.clear();
            }
            Err(e) => {
                tracing::debug!("SMTP password kept in the settings file: {e}");
                fallback = Some(e.to_string());
            }
        }
    } else {
        if let Some(b) = base.as_mut() {
            b.kindle_email.password.clear();
        }
        edited.kindle_email.password.clear();
    }
    let mut settings = match edit_settings(lib, base.as_ref(), &edited) {
        Ok(settings) => settings,
        Err(e) => {
            if let (Some(before), None) = (&before, &fallback) {
                restore(store, before.as_deref());
            }
            return Err(e);
        }
    };
    if settings.kindle_email.password.is_empty() {
        settings.kindle_email.password = password;
    }
    Ok(Saved { settings, fallback })
}

/// The stored settings, after moving a password left in the settings file (saved before the
/// keychain, or while none answered) into the keychain and blanking it in the file. Without a
/// keychain the password stays where it is.
fn migrate(lib: &Library, store: &dyn SecretStore) -> Result<Settings> {
    let settings = get_settings(lib)?;
    let password = &settings.kindle_email.password;
    if password.is_empty() {
        return Ok(settings);
    }
    if let Err(e) = store.set(password) {
        tracing::debug!("SMTP password stays in the settings file: {e}");
        return Ok(settings);
    }
    let mut blank = settings.clone();
    blank.kindle_email.password.clear();
    edit_settings(lib, Some(&settings), &blank)?;
    tracing::info!("moved the SMTP password from the settings file to the system keychain");
    Ok(settings)
}

/// Puts back the keychain's password from before a save that didn't reach the settings.
fn restore(store: &dyn SecretStore, before: Option<&str>) {
    let undone = match before {
        Some(secret) => store.set(secret),
        None => store.delete(),
    };
    if let Err(e) = undone {
        tracing::warn!("couldn't put the previous SMTP password back in the keychain: {e}");
    }
}

fn reveal(store: &dyn SecretStore) -> String {
    store
        .get()
        .unwrap_or_else(|e| {
            tracing::debug!("couldn't read the SMTP password from the system keychain: {e}");
            None
        })
        .unwrap_or_default()
}
