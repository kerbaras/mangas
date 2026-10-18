use serde::Serialize;

use super::format::{Backup, Bookmark, EntryRef, TrackerRef, FORMAT, VERSION};
use crate::domain::Manga;
use crate::library::{is_local, LibrarySnapshot, Settings};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ExportSummary {
    pub titles: usize,
    /// Imported titles left out: their files aren't part of a backup.
    pub skipped_local: usize,
}

/// SQLite's `datetime('now')` text (UTC) as Unix milliseconds; 0 when unreadable.
pub fn created_millis(added_at: &str) -> i64 {
    chrono::NaiveDateTime::parse_from_str(added_at, "%Y-%m-%d %H:%M:%S")
        .map(|t| t.and_utc().timestamp_millis())
        .unwrap_or(0)
}

/// Settings safe to put in a file: no provider sessions (cookies), no SMTP password and no
/// data folder, which belongs to the machine the backup was made on.
pub fn sanitize(settings: &Settings) -> Settings {
    let mut kindle_email = settings.kindle_email.clone();
    kindle_email.password.clear();
    Settings {
        data_dir: None,
        provider_sessions: Default::default(),
        kindle_email,
        ..settings.clone()
    }
}

/// The backup of `entries` (each title with when it was followed, in ms) and their `state`.
/// Imported titles are left out of both.
pub fn build(
    entries: &[(Manga, i64)],
    state: LibrarySnapshot,
    settings: &Settings,
    exported_at: &str,
    app_version: &str,
) -> (Backup, ExportSummary) {
    let (local, remote): (Vec<_>, Vec<_>) = entries.iter().partition(|(m, _)| is_local(&m.source));
    let bookmarks = remote
        .iter()
        .map(|(m, created)| Bookmark {
            created: *created,
            updated: *created,
            title: m.title.clone(),
            media: EntryRef {
                provider_id: m.source.clone(),
                entry_id: m.id.0.clone(),
            },
            info: TrackerRef::default(),
        })
        .collect();
    let backup = Backup {
        format: FORMAT.into(),
        version: VERSION,
        exported_at: exported_at.into(),
        app_version: app_version.into(),
        bookmarks,
        library: remote.iter().map(|(m, _)| m.clone()).collect(),
        settings: Some(sanitize(settings)),
        state: state.only(|source| !is_local(source)),
    };
    let summary = ExportSummary {
        titles: remote.len(),
        skipped_local: local.len(),
    };
    (backup, summary)
}
