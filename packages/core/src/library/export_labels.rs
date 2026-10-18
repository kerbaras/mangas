//! Series, chapter and volume labels for an export, read from the chapters' download folders.

use std::path::Path;

use rusqlite::{params, OptionalExtension};

use super::Library;
use crate::domain::{toc_label, Result};
use crate::export::Labels;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    key: (String, String),
    manga: String,
    series: String,
    title: Option<String>,
    number: String,
    volume: Option<String>,
}

/// Labels for an export of `chapters` folders, looked up by `dirs` (the same folders, possibly
/// spelled several ways; matched like `mark_exported`). Folders outside the library add nothing.
pub fn export_labels(lib: &Library, dirs: &[&Path], chapters: usize) -> Result<Labels> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare(
        "SELECT c.source, c.id, c.manga_id, m.title, c.title, c.number, c.volume
         FROM chapter c JOIN manga m ON m.source = c.source AND m.id = c.manga_id
         WHERE c.local_path IS NOT NULL AND rtrim(c.local_path, '/\\') = ?1",
    )?;
    let mut rows: Vec<Row> = Vec::new();
    for dir in dirs {
        let path = dir.to_string_lossy();
        let found = stmt.query_map(params![path.trim_end_matches(['/', '\\'])], |r| {
            Ok(Row {
                key: (r.get(0)?, r.get(1)?),
                manga: r.get(2)?,
                series: r.get(3)?,
                title: r.get(4)?,
                number: r.get(5)?,
                volume: r.get(6)?,
            })
        })?;
        for row in found {
            let row = row?;
            if !rows.iter().any(|r| r.key == row.key) {
                rows.push(row);
            }
        }
    }
    Ok(labels_of(&rows, chapters))
}

/// Table-of-contents entries for `dirs` (each folder with its possible spellings), in order;
/// empty unless the library knows every folder, so the export falls back to folder names.
pub fn chapter_titles(lib: &Library, dirs: &[Vec<&Path>]) -> Result<Vec<String>> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare(
        "SELECT number, title FROM chapter
         WHERE local_path IS NOT NULL AND rtrim(local_path, '/\\') = ?1 LIMIT 1",
    )?;
    let mut out = Vec::with_capacity(dirs.len());
    for spellings in dirs {
        let found = spellings
            .iter()
            .find_map(|dir| {
                let path = dir.to_string_lossy();
                stmt.query_row(params![path.trim_end_matches(['/', '\\'])], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
                })
                .optional()
                .transpose()
            })
            .transpose()?;
        match found {
            Some((number, title)) => out.push(toc_label(&number, title.as_deref())),
            None => return Ok(Vec::new()),
        }
    }
    Ok(out)
}

/// Each label is kept only when it holds for every exported chapter; a volume needs one series.
fn labels_of(rows: &[Row], chapters: usize) -> Labels {
    let all_known = !rows.is_empty() && rows.len() == chapters;
    let single = all_known && chapters == 1;
    let shared = |f: fn(&Row) -> Option<&str>| {
        let first = f(rows.first()?)?;
        rows.iter()
            .all(|r| f(r) == Some(first))
            .then(|| first.to_owned())
    };
    let some = |s: &str| (!s.trim().is_empty()).then(|| s.to_owned());
    let one_manga = rows
        .iter()
        .all(|r| (&r.key.0, &r.manga) == (&rows[0].key.0, &rows[0].manga));
    let series = shared(|r| Some(&r.series)).filter(|_| all_known && one_manga);
    let one = |f: fn(&Row) -> Option<&str>| rows.first().and_then(f).and_then(some);
    Labels {
        title: one(|r| r.title.as_deref()).filter(|_| single),
        number: one(|r| Some(&r.number)).filter(|_| single),
        volume: shared(|r| r.volume.as_deref())
            .and_then(|v| some(&v))
            .filter(|_| series.is_some()),
        series,
    }
}
