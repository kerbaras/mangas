//! The one slot the SMTP password lives in: the OS keychain, or memory (tests, keychain off).

use std::sync::Mutex;

use super::memo::LockMemo;

/// Why the keychain didn't answer; the text is the platform's own description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// There is a keychain but it won't open: locked, or its unlock prompt was dismissed.
    Locked(String),
    /// No keychain to use: none on this computer, turned off, or it failed.
    Unavailable(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Locked(why) | StoreError::Unavailable(why) => f.write_str(why),
        }
    }
}

/// A single secret.
pub trait SecretStore: Send + Sync {
    /// The stored secret; `None` when there is none.
    fn get(&self) -> Result<Option<String>, StoreError>;
    fn set(&self, secret: &str) -> Result<(), StoreError>;
    /// Removes the secret; removing a missing one is not an error.
    fn delete(&self) -> Result<(), StoreError>;
}

/// Keychain service and account names. The service is the app identifier, as macOS shows it.
const SERVICE: &str = "dev.kerbaras.mangas";
const ACCOUNT: &str = "smtp-password";

/// Setting this variable to `off` (or `0`) makes [`system_store`] act as if there were no
/// keychain: the password stays in the app settings. Tests use it to keep off the real one.
pub const KEYCHAIN_ENV: &str = "MANGAS_KEYCHAIN";

/// The keychain said it is locked (or its unlock prompt was dismissed): later reads in this
/// process answer that without prompting, until [`retry_keychain`].
static KEYCHAIN_LOCK: LockMemo = LockMemo::new();

/// Lets the next keychain read prompt to unlock again: call it only for something the user did
/// (opened the email settings, clicked the password field, sent a book).
pub fn retry_keychain() {
    KEYCHAIN_LOCK.forget();
}

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

fn entry() -> Result<keyring::Entry, StoreError> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(describe)
}

fn describe(e: keyring::Error) -> StoreError {
    match e {
        keyring::Error::NoDefaultStore => {
            StoreError::Unavailable("no system keychain on this computer".to_string())
        }
        e @ keyring::Error::NoStorageAccess(_) => StoreError::Locked(e.to_string()),
        e => StoreError::Unavailable(e.to_string()),
    }
}

impl SecretStore for Keychain {
    fn get(&self) -> Result<Option<String>, StoreError> {
        KEYCHAIN_LOCK.read(|| match entry()?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(describe(e)),
        })
    }

    fn set(&self, secret: &str) -> Result<(), StoreError> {
        KEYCHAIN_LOCK.write(|| entry()?.set_password(secret).map_err(describe))
    }

    fn delete(&self) -> Result<(), StoreError> {
        KEYCHAIN_LOCK.write(|| match entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(describe(e)),
        })
    }
}

/// A secret held in memory, or a store that always fails (no keychain).
#[derive(Default)]
pub struct MemoryStore {
    secret: Mutex<Option<String>>,
    broken: Option<StoreError>,
}

impl MemoryStore {
    /// A store holding `secret`.
    pub fn with(secret: &str) -> Self {
        MemoryStore {
            secret: Mutex::new(Some(secret.to_string())),
            broken: None,
        }
    }

    /// A store whose every call fails with `reason`, as without a keychain.
    pub fn unavailable(reason: &str) -> Self {
        MemoryStore {
            secret: Mutex::new(None),
            broken: Some(StoreError::Unavailable(reason.to_string())),
        }
    }

    /// A store whose every call fails with `reason`, as a locked keychain.
    pub fn locked(reason: &str) -> Self {
        MemoryStore {
            secret: Mutex::new(None),
            broken: Some(StoreError::Locked(reason.to_string())),
        }
    }

    /// What is stored, bypassing the failure mode.
    pub fn peek(&self) -> Option<String> {
        self.secret.lock().ok().and_then(|s| s.clone())
    }

    fn slot(&self) -> Result<std::sync::MutexGuard<'_, Option<String>>, StoreError> {
        if let Some(reason) = &self.broken {
            return Err(reason.clone());
        }
        self.secret
            .lock()
            .map_err(|_| StoreError::Unavailable("secret store poisoned".to_string()))
    }
}

impl SecretStore for MemoryStore {
    fn get(&self) -> Result<Option<String>, StoreError> {
        Ok(self.slot()?.clone())
    }

    fn set(&self, secret: &str) -> Result<(), StoreError> {
        *self.slot()? = Some(secret.to_string());
        Ok(())
    }

    fn delete(&self) -> Result<(), StoreError> {
        *self.slot()? = None;
        Ok(())
    }
}
