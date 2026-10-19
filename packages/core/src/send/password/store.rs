//! The one slot the SMTP password lives in: the OS keychain, or memory (tests, keychain off).

use std::sync::Mutex;

/// A single secret. Errors are the platform's own description of what failed.
pub trait SecretStore: Send + Sync {
    /// The stored secret; `None` when there is none.
    fn get(&self) -> Result<Option<String>, String>;
    fn set(&self, secret: &str) -> Result<(), String>;
    /// Removes the secret; removing a missing one is not an error.
    fn delete(&self) -> Result<(), String>;
}

/// Keychain service and account names. The service is the app identifier, as macOS shows it.
const SERVICE: &str = "dev.kerbaras.mangas";
const ACCOUNT: &str = "smtp-password";

/// Setting this variable to `off` (or `0`) makes [`system_store`] act as if there were no
/// keychain: the password stays in the app settings. Tests use it to keep off the real one.
pub const KEYCHAIN_ENV: &str = "MANGAS_KEYCHAIN";

/// macOS Keychain, Windows Credential Manager or the Secret Service (GNOME Keyring, KWallet).
pub struct Keychain;

/// The keychain, unless [`KEYCHAIN_ENV`] turns it off.
pub fn system_store() -> Box<dyn SecretStore> {
    let off = std::env::var(KEYCHAIN_ENV).is_ok_and(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "off" | "0" | "false"
        )
    });
    if off {
        Box::new(MemoryStore::unavailable(&format!(
            "system keychain turned off by {KEYCHAIN_ENV}"
        )))
    } else {
        Box::new(Keychain)
    }
}

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(describe)
}

fn describe(e: keyring::Error) -> String {
    match e {
        keyring::Error::NoDefaultStore => "no system keychain on this computer".to_string(),
        e => e.to_string(),
    }
}

impl SecretStore for Keychain {
    fn get(&self) -> Result<Option<String>, String> {
        match entry()?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(describe(e)),
        }
    }

    fn set(&self, secret: &str) -> Result<(), String> {
        entry()?.set_password(secret).map_err(describe)
    }

    fn delete(&self) -> Result<(), String> {
        match entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(describe(e)),
        }
    }
}

/// A secret held in memory, or a store that always fails (no keychain).
#[derive(Default)]
pub struct MemoryStore {
    secret: Mutex<Option<String>>,
    broken: Option<String>,
}

impl MemoryStore {
    /// A store holding `secret`.
    pub fn with(secret: &str) -> Self {
        MemoryStore {
            secret: Mutex::new(Some(secret.to_string())),
            broken: None,
        }
    }

    /// A store whose every call fails with `reason`.
    pub fn unavailable(reason: &str) -> Self {
        MemoryStore {
            secret: Mutex::new(None),
            broken: Some(reason.to_string()),
        }
    }

    /// What is stored, bypassing the failure mode.
    pub fn peek(&self) -> Option<String> {
        self.secret.lock().ok().and_then(|s| s.clone())
    }

    fn slot(&self) -> Result<std::sync::MutexGuard<'_, Option<String>>, String> {
        if let Some(reason) = &self.broken {
            return Err(reason.clone());
        }
        self.secret
            .lock()
            .map_err(|_| "secret store poisoned".to_string())
    }
}

impl SecretStore for MemoryStore {
    fn get(&self) -> Result<Option<String>, String> {
        Ok(self.slot()?.clone())
    }

    fn set(&self, secret: &str) -> Result<(), String> {
        *self.slot()? = Some(secret.to_string());
        Ok(())
    }

    fn delete(&self) -> Result<(), String> {
        *self.slot()? = None;
        Ok(())
    }
}
