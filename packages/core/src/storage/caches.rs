use serde::{Deserialize, Serialize};

use super::tree::empty_dir;
use super::Folders;
use crate::domain::Result;
use crate::ml::lease::{self, Purpose};

/// A folder the app can rebuild on its own, so clearing it loses nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cache {
    /// Cover thumbnails, fetched again when a cover is shown.
    Covers,
    /// Detector models, downloaded again on the next smart-cut export that uses them.
    Models,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Cleared {
    pub files: usize,
    pub bytes: u64,
}

/// Deletes everything inside the cache folder (the folder itself stays).
pub fn clear_cache(folders: &Folders, cache: Cache) -> Result<Cleared> {
    let dir = match cache {
        Cache::Covers => folders.covers.clone(),
        Cache::Models => folders.models(),
    };
    if !dir.exists() {
        return Ok(Cleared::default());
    }
    // Models: not under a detector download, move or export, in this process or another.
    let (_lease, keep) = match cache {
        Cache::Models => (
            Some(lease::acquire(&dir, Purpose::Remove)?),
            &lease::LEASE_FILES[..],
        ),
        Cache::Covers => (None, &[][..]),
    };
    let (files, bytes) = empty_dir(&dir, keep)?;
    Ok(Cleared { files, bytes })
}
