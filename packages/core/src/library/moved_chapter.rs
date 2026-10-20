//! Chapter folders copied or moved out of the downloads folder, matched back to the library by
//! the `<source>/<manga id>/<chapter>` names a download gives them (`download::chapter_dir`).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use rusqlite::params;

use super::Library;
use crate::domain::Result;

/// The last `n` names of `path` (made absolute), innermost last; fewer when it is shorter.
fn tail(path: &Path, n: usize) -> Vec<OsString> {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let names: Vec<OsString> = path.iter().map(OsString::from).collect();
    names[names.len().saturating_sub(n)..].to_vec()
}

/// The stored download folder(s) of the chapter `spellings` (one folder, spelled as typed,
/// absolute, resolved) is a copy of, for a folder the library does not store: the stored folders
/// whose `<manga id>/<chapter>` names match, narrowed to the same `<source>` folder above when
/// several series match. Empty when the folder is stored itself, or no single series matches.
pub fn stored_original(lib: &Library, spellings: &[&Path]) -> Result<Vec<PathBuf>> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare(
        "SELECT source, manga_id, rtrim(local_path, '/\\') FROM chapter
         WHERE local_path IS NOT NULL",
    )?;
    let rows: Vec<(String, String, String)> = stmt
        .query_map(params![], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let given: Vec<PathBuf> = spellings
        .iter()
        .map(|s| PathBuf::from(s.to_string_lossy().trim_end_matches(['/', '\\'])))
        .collect();
    if rows
        .iter()
        .any(|(_, _, p)| given.iter().any(|g| g == Path::new(p)))
    {
        return Ok(Vec::new());
    }
    let Some(first) = given.first() else {
        return Ok(Vec::new());
    };
    let names = tail(first, 3);
    if names.len() < 2 {
        return Ok(Vec::new());
    }
    let matches: Vec<&(String, String, String)> = rows
        .iter()
        .filter(|(_, _, p)| tail(Path::new(p), 2) == names[names.len() - 2..])
        .collect();
    Ok(one_series(
        &matches,
        names.first().filter(|_| names.len() == 3),
    ))
}

/// The paths of `matches` when they are one series, else those under a `source` folder named
/// like the copy's, when that leaves one series.
fn one_series(matches: &[&(String, String, String)], source: Option<&OsString>) -> Vec<PathBuf> {
    let series = |m: &[&(String, String, String)]| {
        let mut keys: Vec<(&str, &str)> = m.iter().map(|r| (r.0.as_str(), r.1.as_str())).collect();
        keys.sort();
        keys.dedup();
        keys.len()
    };
    let narrowed: Vec<&(String, String, String)>;
    let picked = match (series(matches), source) {
        (1, _) => matches,
        (_, Some(source)) => {
            let folder = |p: &str| tail(Path::new(p), 3).first().cloned();
            narrowed = matches
                .iter()
                .copied()
                .filter(|r| folder(&r.2).as_ref() == Some(source))
                .collect();
            if series(&narrowed) == 1 {
                &narrowed[..]
            } else {
                &[]
            }
        }
        _ => &[],
    };
    picked.iter().map(|r| PathBuf::from(&r.2)).collect()
}

/// The series folder name of chapter folder `dir` for a book title, unless it is a manga id:
/// the folder sits in a `<source>` folder named after one of `sources` (the download layout).
pub fn series_folder_name(dir: &Path, sources: &[&str]) -> Option<String> {
    let dir = dir.canonicalize().ok()?;
    let series = dir.parent()?;
    let source = series.parent()?.file_name()?.to_string_lossy();
    if sources.iter().any(|s| *s == source) {
        return None;
    }
    Some(series.file_name()?.to_string_lossy().into_owned())
}
