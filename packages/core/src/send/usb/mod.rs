//! Send to Kindle over USB: find a Kindle mounted as a drive (or over MTP through gvfs on
//! Linux) and copy the book into its `documents/` folder.

mod copy;
mod detect;

use std::path::Path;

pub use copy::{copy_to_kindle, Copied};
pub use detect::{
    find_kindles, find_kindles_in, kindle_at, looks_like_kindle, mount_roots, KindleVolume,
};

use crate::domain::{Error, Result};

/// The Kindle mounted at `root` when given (any folder with `documents/`), otherwise the first
/// one found among `found`.
pub fn choose(root: Option<&Path>, found: Vec<KindleVolume>) -> Result<KindleVolume> {
    let picked = match root {
        Some(root) => kindle_at(root),
        None => found.into_iter().next(),
    };
    picked.ok_or_else(|| {
        Error::Send(match root {
            Some(root) => format!(
                "Device not found: {} has no documents folder",
                root.display()
            ),
            None => "Device not found: connect your Kindle over USB and unlock it".to_string(),
        })
    })
}
