//! The SMTP password, kept in the OS keychain rather than in the app settings. A computer
//! without one (a headless Linux with no Secret Service) keeps it in the settings file, as
//! before, and the caller tells the user once, when it is saved.

mod settings;
mod store;

pub use settings::{
    email_settings, load_settings, save_settings, storage, with_password, Saved, Storage,
};
pub use store::{system_store, Keychain, MemoryStore, SecretStore, StoreError, KEYCHAIN_ENV};
