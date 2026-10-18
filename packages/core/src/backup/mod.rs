//! Library backup: the followed titles, their user state (categories, reading status, chapter
//! marks) and preferences as one JSON file, minus secrets and downloaded files.
//! Its `bookmarks` use HaruNeko's bookmark shape, so either app can read the other's list.

mod export;
mod format;
mod restore;
mod titles;

use std::fs;
use std::path::Path;

pub use export::{build, created_millis, sanitize, ExportSummary};
pub use format::{parse, Backup, Bookmark, EntryRef, TrackerRef, FORMAT, VERSION};
pub use restore::{merge_settings, plan, restore, Restored, Skipped};
pub use titles::{sqlite_time, title_state};

use crate::domain::{Error, Result};
use crate::library::{get_settings, list_manga_with_added, read_snapshot, Library};

/// Writes a backup of `lib` to `path` (through a temporary file, so a failed write never
/// leaves half a backup behind).
pub fn export_to(lib: &Library, path: &Path, app_version: &str) -> Result<ExportSummary> {
    let entries: Vec<_> = list_manga_with_added(lib)?
        .into_iter()
        .map(|(manga, added)| (manga, created_millis(&added)))
        .collect();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let state = read_snapshot(lib)?;
    let (backup, summary) = build(&entries, state, &get_settings(lib)?, &now, app_version);
    let json =
        serde_json::to_string_pretty(&backup).map_err(|e| Error::Other(format!("backup: {e}")))?;
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".part");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)?;
    Ok(summary)
}

/// Reads a mangas backup or a HaruNeko bookmarks export from `path` and merges it into `lib`.
/// `known_sources` are the provider ids this build supports.
pub fn import_from(lib: &Library, path: &Path, known_sources: &[&str]) -> Result<Restored> {
    let backup = parse(&fs::read_to_string(path)?)?;
    restore(lib, &backup, known_sources)
}
