use std::collections::HashSet;

use serde::Serialize;

use super::format::Backup;
use crate::domain::{Manga, MangaId, Result};
use crate::library::{get_settings, is_local, list_manga, set_settings, upsert_manga};
use crate::library::{Library, Settings};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Restored {
    pub added: usize,
    /// Titles already followed; kept as they are.
    pub already: usize,
    pub skipped: Vec<Skipped>,
    pub settings: bool,
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

/// Restored preferences, keeping this machine's provider sessions and data folder.
pub fn merge_settings(current: &Settings, restored: &Settings) -> Settings {
    Settings {
        data_dir: current.data_dir.clone(),
        provider_sessions: current.provider_sessions.clone(),
        ..restored.clone()
    }
}

/// Follows the backup's new titles and applies its preferences.
pub fn restore(lib: &Library, backup: &Backup, known_sources: &[&str]) -> Result<Restored> {
    let followed = list_manga(lib)?
        .into_iter()
        .map(|m| (m.source, m.id.0))
        .collect();
    let (add, mut report) = plan(backup, &followed, known_sources);
    for manga in &add {
        upsert_manga(lib, manga)?;
    }
    if let Some(restored) = &backup.settings {
        set_settings(lib, &merge_settings(&get_settings(lib)?, restored))?;
        report.settings = true;
    }
    Ok(report)
}
