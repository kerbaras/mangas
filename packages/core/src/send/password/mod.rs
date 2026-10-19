//! The SMTP password, kept in the OS keychain rather than in the app settings. A computer
//! without one (a headless Linux with no Secret Service) keeps it in the settings file, as
//! before, and the caller tells the user once, when it is saved.

mod memo;
mod resolve;
mod settings;
mod store;
mod view;

pub use resolve::{reads_keychain, test_settings, with_password};
pub use settings::{email_settings, load_settings, save_settings, storage, Saved, Storage};
pub use store::{
    retry_keychain, system_store, Keychain, MemoryStore, SecretStore, StoreError, KEYCHAIN_ENV,
};
pub use view::{has_password, public_settings, redact, save_preferences, set_password};
