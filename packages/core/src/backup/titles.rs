use super::format::Backup;
use crate::domain::Manga;
use crate::library::TitleState;

/// Unix milliseconds as SQLite's `datetime('now')` text; `None` for 0 or out-of-range values.
pub fn sqlite_time(millis: i64) -> Option<String> {
    (millis > 0)
        .then(|| chrono::DateTime::from_timestamp_millis(millis))
        .flatten()
        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
}

/// The state a restored title starts with: the backup's own, or for a backup without one
/// (before 0.3.0, or a HaruNeko list) the defaults, followed when its bookmark was created.
pub fn title_state(backup: &Backup, manga: &Manga) -> TitleState {
    let same = |source: &str, id: &str| source == manga.source && id == manga.id.0;
    if let Some(state) = backup.state.titles.iter().find(|t| same(&t.source, &t.id)) {
        return state.clone();
    }
    let created = backup
        .bookmarks
        .iter()
        .find(|b| same(&b.media.provider_id, &b.media.entry_id))
        .and_then(|b| sqlite_time(b.created));
    TitleState {
        source: manga.source.clone(),
        id: manga.id.0.clone(),
        added_at: created,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bookmark_times_become_sqlite_text() {
        assert_eq!(
            sqlite_time(1_792_311_548_000).as_deref(),
            Some("2026-10-18 08:19:08")
        );
        assert_eq!(sqlite_time(0), None);
        assert_eq!(sqlite_time(-5), None);
    }
}
