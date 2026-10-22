//! Who a book of chapter folders credits, read from the series those folders belong to.

use std::path::Path;

use rusqlite::params;

use super::{get_manga, Library};
use crate::domain::{MangaId, Result};

/// The creators ([`crate::domain::Manga::creators`]) of every series among `dirs` (each folder
/// with its possible spellings), in folder order and each once. Folders outside the library add
/// nobody, so an unknown or uncredited series gives an empty list, never a placeholder.
pub fn export_creators(lib: &Library, dirs: &[Vec<&Path>]) -> Result<Vec<String>> {
    let series = {
        let conn = lib.conn()?;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT source, manga_id FROM chapter
             WHERE local_path IS NOT NULL AND rtrim(local_path, '/\\') = ?1",
        )?;
        let mut series: Vec<(String, String)> = Vec::new();
        for dir in dirs.iter().flatten() {
            let path = dir.to_string_lossy();
            let rows = stmt.query_map(params![path.trim_end_matches(['/', '\\'])], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
            for row in rows {
                let row = row?;
                if !series.contains(&row) {
                    series.push(row);
                }
            }
        }
        series
    };
    let mut out: Vec<String> = Vec::new();
    for (source, id) in series {
        let Some(manga) = get_manga(lib, &source, &MangaId(id))? else {
            continue;
        };
        for name in manga.creators() {
            if !out.contains(&name) {
                out.push(name);
            }
        }
    }
    Ok(out)
}
