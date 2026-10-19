use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::format::Backup;
use super::titles::title_state;
use crate::domain::{Manga, MangaId, Result};
use crate::library::{edit_settings, get_settings, is_local, list_manga, restore_snapshot};
use crate::library::{Library, Settings};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Restored {
    pub added: usize,
    /// Titles already followed; kept as they are.
    pub already: usize,
    pub skipped: Vec<Skipped>,
    /// The backup's preferences were applied.
    pub settings: bool,
    /// The backup had preferences, and yours were kept instead.
    pub kept_settings: bool,
}

/// Whose preferences a restore ends up with. `Auto` keeps yours when the library already has
/// titles (merging a backup in) and takes the backup's into an empty one (a new machine).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preferences {
    #[default]
    Auto,
    Keep,
    Backup,
}

impl Preferences {
    pub fn keeps_current(self, library_empty: bool) -> bool {
        match self {
            Preferences::Auto => !library_empty,
            Preferences::Keep => true,
            Preferences::Backup => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skipped {
    pub title: String,
    pub reason: String,
}

/// The titles to follow and the report so far. A bookmark with stored metadata restores it;
/// a bare HaruNeko one restores the id and title, and the rest arrives on the next refresh.
pub fn plan(
    backup: &Backup,
    followed: &HashSet<(String, String)>,
    known_sources: &[&str],
) -> (Vec<Manga>, Restored) {
    let mut seen = followed.clone();
    let mut report = Restored::default();
    let mut add = Vec::new();
    for b in &backup.bookmarks {
        let (source, id) = (&b.media.provider_id, &b.media.entry_id);
        if is_local(source) || !known_sources.contains(&source.as_str()) {
            report.skipped.push(Skipped {
                title: b.title.clone(),
                reason: format!("{source} isn't a provider mangas supports"),
            });
            continue;
        }
        if !seen.insert((source.clone(), id.clone())) {
            report.already += 1;
            continue;
        }
        let stored = backup
            .library
            .iter()
            .find(|m| &m.source == source && &m.id.0 == id);
        add.push(stored.cloned().unwrap_or_else(|| Manga {
            id: MangaId(id.clone()),
            source: source.clone(),
            title: b.title.clone(),
            ..Default::default()
        }));
    }
    report.added = add.len();
    (add, report)
}

/// Restored preferences, keeping this machine's provider sessions, data folder and SMTP
/// password (backups never carry one).
pub fn merge_settings(current: &Settings, restored: &Settings) -> Settings {
    let mut kindle_email = restored.kindle_email.clone();
    kindle_email.password = current.kindle_email.password.clone();
    Settings {
        data_dir: current.data_dir.clone(),
        provider_sessions: current.provider_sessions.clone(),
        kindle_email,
        ..restored.clone()
    }
}

/// Follows the backup's new titles with their state, adds categories, assignments and chapter
/// marks to the titles followed afterwards, and, unless the library already has titles, applies
/// the backup's preferences; see [`restore_with`].
pub fn restore(lib: &Library, backup: &Backup, known_sources: &[&str]) -> Result<Restored> {
    restore_with(lib, backup, known_sources, Preferences::Auto)
}

/// Follows the backup's new titles, then applies its preferences or keeps the current ones.
pub fn restore_with(
    lib: &Library,
    backup: &Backup,
    known_sources: &[&str],
    preferences: Preferences,
) -> Result<Restored> {
    let followed: HashSet<_> = list_manga(lib)?
        .into_iter()
        .map(|m| (m.source, m.id.0))
        .collect();
    let keep = preferences.keeps_current(followed.is_empty());
    let (add, mut report) = plan(backup, &followed, known_sources);
    let add: Vec<_> = add
        .into_iter()
        .map(|m| {
            let state = title_state(backup, &m);
            (m, state)
        })
        .collect();
    // Read the settings before any write: an unreadable row refuses the whole restore up front.
    let current = match &backup.settings {
        Some(_) if !keep => Some(get_settings(lib)?),
        _ => None,
    };
    restore_snapshot(lib, &add, &backup.state)?;
    match (&backup.settings, current) {
        (Some(_), _) if keep => report.kept_settings = true,
        (Some(restored), Some(current)) => {
            // Applied over what is stored now, so edits made meanwhile in another process survive.
            edit_settings(lib, Some(&current), &merge_settings(&current, restored))?;
            report.settings = true;
        }
        _ => {}
    }
    Ok(report)
}
