//! Marks on stored chapters: where a download lives and when it was last exported.

use std::path::Path;

use rusqlite::params;

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
