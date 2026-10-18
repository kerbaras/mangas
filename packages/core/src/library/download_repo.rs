//! Chapter folders the library records on disk (`chapter.local_path`).

use rusqlite::params;

use super::settings_repo::{encode, KEY};
use super::{Library, Settings};
use crate::domain::Result;

/// A downloaded chapter as storage management sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedChapter {
    pub source: String,
    pub manga_id: String,
    pub manga_title: String,
    pub chapter_id: String,
    pub path: String,
}

/// Every chapter with a `local_path`, optionally limited to one manga.
pub fn downloaded_chapters(
    lib: &Library,
    manga: Option<(&str, &str)>,
) -> Result<Vec<DownloadedChapter>> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare(
        "SELECT c.source, c.manga_id, m.title, c.id, c.local_path
         FROM chapter c JOIN manga m ON m.source = c.source AND m.id = c.manga_id
         WHERE c.local_path IS NOT NULL
           AND (?1 IS NULL OR (c.source = ?1 AND c.manga_id = ?2))
         ORDER BY c.source, c.manga_id, c.rowid",
    )?;
    let (source, id) = manga.unzip();
    let rows = stmt.query_map(params![source, id], |r| {
        Ok(DownloadedChapter {
            source: r.get(0)?,
            manga_id: r.get(1)?,
            manga_title: r.get(2)?,
            chapter_id: r.get(3)?,
            path: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Forgets that a chapter is on disk; its metadata stays so it can be downloaded again.
pub fn forget_download(lib: &Library, source: &str, chapter_id: &str) -> Result<()> {
    lib.conn()?.execute(
        "UPDATE chapter SET local_path = NULL WHERE source = ?1 AND id = ?2",
        params![source, chapter_id],
    )?;
    Ok(())
}

/// Points chapters at their new folders and saves `settings`,
/// in one transaction, so a data-folder move either fully switches or not at all.
pub(crate) fn switch_data_dir(
    lib: &Library,
    moved: &[(DownloadedChapter, String)],
    settings: &Settings,
) -> Result<()> {
    let mut conn = lib.conn()?;
    let tx = conn.transaction()?;
    {
        let mut stmt =
            tx.prepare("UPDATE chapter SET local_path = ?3 WHERE source = ?1 AND id = ?2")?;
        for (chapter, path) in moved {
            stmt.execute(params![chapter.source, chapter.chapter_id, path])?;
        }
    }
    tx.execute(
        "INSERT INTO setting (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![KEY, encode(settings)?],
    )?;
    tx.commit()?;
    Ok(())
}
