//! App self-updates: which manifest each release channel reads, and whether its version should
//! replace the running one. The desktop installs through `tauri-plugin-updater` with these
//! endpoints and [`should_update`]; `mangas update --check` runs [`check`] on the same rules.

mod channel;
mod check;
mod manifest;
mod signature;
mod version;

pub use channel::{endpoints, Channel, CHANNELS_TAG, OVERRIDE_ENV, RELEASES};
pub use check::{check, override_url};
pub use manifest::{current_target, evaluate, parse_manifest, Manifest, Status};
pub use semver::Version;
pub use signature::{key_id, pubkey_from_config, signed_by};
pub use version::{is_older, parse_version, should_update};
