//! Which languages of a followed title have been synced at least once. The first sync of a
//! language is the baseline; only chapters that appear in later syncs are stamped as new.

use rusqlite::{params, Transaction};

use crate::domain::{MangaId, Result};

pub(super) fn synced_before(
    tx: &Transaction,
    source: &str,
    manga_id: &MangaId,
    language: &str,
) -> Result<bool> {
    Ok(tx.query_row(
        "SELECT EXISTS (SELECT 1 FROM manga_language_sync
                        WHERE source = ?1 AND manga_id = ?2 AND language = ?3)",
        params![source, manga_id.0, language],
        |r| r.get(0),
    )?)
}

pub(super) fn record(
    tx: &Transaction,
    source: &str,
    manga_id: &MangaId,
    language: &str,
) -> Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO manga_language_sync (source, manga_id, language)
         SELECT source, id, ?3 FROM manga WHERE source = ?1 AND id = ?2",
        params![source, manga_id.0, language],
    )?;
    Ok(())
}
