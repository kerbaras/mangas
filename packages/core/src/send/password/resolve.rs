//! The SMTP password a send uses: the one in the email settings, else the keychain's. One
//! place for the CLI, the app's sends and its test email.

use super::store::{SecretStore, StoreError};
use crate::domain::{Error, Result};
use crate::send::EmailSettings;

/// Whether sending with `email` needs the keychain's password: a user name and no password.
pub fn reads_keychain(email: &EmailSettings) -> bool {
    email.password.is_empty() && !email.username.trim().is_empty()
}

/// `email` with its password: the one it carries (one still in the settings file), else the
/// keychain's. Without a user name no password is needed; without a keychain the password is
/// empty, as before the keychain, and the server says whether it takes that. A locked keychain
/// (its unlock prompt dismissed) fails the send instead of trying without the password.
pub fn with_password(store: &dyn SecretStore, email: &EmailSettings) -> Result<EmailSettings> {
    if !reads_keychain(email) {
        return Ok(email.clone());
    }
    let password = match store.get() {
        Ok(secret) => secret.unwrap_or_default(),
        Err(StoreError::Locked(why)) => return Err(locked(&why)),
        Err(e) => {
            tracing::debug!("couldn't read the SMTP password from the system keychain: {e}");
            String::new()
        }
    };
    Ok(EmailSettings {
        password,
        ..email.clone()
    })
}

/// The settings a test email uses: the form's `draft`, with the saved password when the user
/// didn't change it (`typed` false; a locked keychain fails like a send), else exactly as typed,
/// so a cleared field tests sending without one. Only a keychain read may prompt to unlock.
pub fn test_settings(
    store: &dyn SecretStore,
    draft: &EmailSettings,
    typed: bool,
) -> Result<EmailSettings> {
    if typed {
        return Ok(draft.clone());
    }
    if reads_keychain(draft) {
        store.retry();
    }
    with_password(store, draft)
}

pub(super) fn locked(why: &str) -> Error {
    Error::Send(format!(
        "The system keychain is locked or can't be read, so the SMTP password isn't available. \
         Unlock it and try again ({why})"
    ))
}
