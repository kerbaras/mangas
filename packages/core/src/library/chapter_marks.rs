//! Marks on stored chapters: where a download lives and when it was last exported.

use std::path::Path;

use rusqlite::params;
use serde::Serialize;

use super::Library;
use crate::domain::{ChapterId, Result};

/// Records where a chapter's pages live on disk.
pub fn mark_downloaded(
    lib: &Library,
    source: &str,
    chapter: &ChapterId,
    path: &Path,
) -> Result<()> {
    lib.conn()?.execute(
        "UPDATE chapter SET local_path = ?3 WHERE source = ?1 AND id = ?2",
        params![source, chapter.0, path.to_string_lossy()],
    )?;
    Ok(())
}

/// Records that the chapters downloaded to `dirs` were just exported. Paths match the stored
/// `local_path` ignoring a trailing separator; folders that aren't library chapters are skipped.
/// Returns how many chapter rows were marked.
pub fn mark_exported(lib: &Library, dirs: &[&Path]) -> Result<usize> {
    let mut conn = lib.conn()?;
    let tx = conn.transaction()?;
    let mut marked = 0;
    {
        let mut stmt = tx.prepare(
            "UPDATE chapter SET exported_at = datetime('now')
             WHERE local_path IS NOT NULL AND rtrim(local_path, '/\\') = ?1",
        )?;
        for dir in dirs {
            let path = dir.to_string_lossy();
            marked += stmt.execute(params![path.trim_end_matches(['/', '\\'])])?;
        }
    }
    tx.commit()?;
    Ok(marked)
}

/// A library chapter an export just marked as exported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportedChapter {
    /// The series' title in the library.
    pub manga: String,
    /// The chapter's number, as the source gives it.
    pub chapter: String,
}

/// Marks the chapter stored at one of `spellings` (one folder, spelled the ways the library may
/// store it) as just exported and returns it; `None` when no library chapter is stored there.
pub fn mark_exported_folder(lib: &Library, spellings: &[&Path]) -> Result<Option<ExportedChapter>> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare(
        "UPDATE chapter SET exported_at = datetime('now')
         WHERE local_path IS NOT NULL AND rtrim(local_path, '/\\') = ?1
         RETURNING number, (SELECT title FROM manga m
                            WHERE m.source = chapter.source AND m.id = chapter.manga_id)",
    )?;
    let mut found = Vec::new();
    for path in spellings {
        let path = path.to_string_lossy();
        let rows = stmt.query_map(params![path.trim_end_matches(['/', '\\'])], |r| {
            Ok(ExportedChapter {
                chapter: r.get(0)?,
                manga: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
            })
        })?;
        found.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
    }
    Ok(found.into_iter().next())
}
