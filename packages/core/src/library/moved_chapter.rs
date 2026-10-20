//! Chapter folders copied or moved out of the downloads folder, matched back to the library by
//! the `<source>/<manga id>/<chapter>` names a download gives them (`download::chapter_dir`).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use super::Library;
use crate::domain::Result;

/// The last `n` names of `path` (made absolute), innermost last; fewer when it is shorter.
fn tail(path: &Path, n: usize) -> Vec<OsString> {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let names: Vec<OsString> = path.iter().map(OsString::from).collect();
    names[names.len().saturating_sub(n)..].to_vec()
}

type Row = (String, String, String);

/// The stored download folder(s) of the chapter `spellings` (one folder, spelled as typed,
/// absolute, resolved) is a copy of, for a folder the library does not store: the stored folders
/// whose `<manga id>/<chapter>` names match. A copy in a folder named after one of `sources`
/// matches only that source's folders. The last spelling (resolved) is tried first: it is where
/// the files really are. Empty when the folder is stored itself, or no single series matches.
pub fn stored_original(
    lib: &Library,
    spellings: &[&Path],
    sources: &[&str],
) -> Result<Vec<PathBuf>> {
    let conn = lib.conn()?;
    let given: Vec<String> = spellings
        .iter()
        .map(|s| {
            s.to_string_lossy()
                .trim_end_matches(['/', '\\'])
                .to_string()
        })
        .collect();
    let mut stored = conn.prepare(
        "SELECT EXISTS(SELECT 1 FROM chapter WHERE local_path IN (?1, ?1 || '/', ?1 || '\\'))",
    )?;
    for g in &given {
        if stored.query_row(params![g], |r| r.get::<_, bool>(0))? {
            return Ok(Vec::new());
        }
    }
    for g in given.iter().rev() {
        let found = copy_of(&conn, Path::new(g), sources)?;
        if !found.is_empty() {
            return Ok(found);
        }
    }
    Ok(Vec::new())
}

/// [`stored_original`] for one spelling `dir`.
fn copy_of(conn: &Connection, dir: &Path, sources: &[&str]) -> Result<Vec<PathBuf>> {
    let names = tail(dir, 3);
    if names.len() < 2 {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT source, manga_id, rtrim(local_path, '/\\') FROM chapter
         WHERE local_path IS NOT NULL AND instr(local_path, ?1) > 0",
    )?;
    let series_folder = names[names.len() - 2].to_string_lossy().into_owned();
    let rows: Vec<Row> = stmt
        .query_map(params![series_folder], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let source = names
        .first()
        .filter(|s| names.len() == 3 && sources.iter().any(|known| s.as_os_str() == *known));
    let matches: Vec<&Row> = rows
        .iter()
        .filter(|(_, _, p)| tail(Path::new(p), 2) == names[names.len() - 2..])
        .filter(|(_, _, p)| source.is_none() || tail(Path::new(p), 3).first() == source)
        .collect();
    Ok(one_series(&matches))
}

/// The paths of `matches` when they all belong to one series.
fn one_series(matches: &[&Row]) -> Vec<PathBuf> {
    let mut keys: Vec<(&str, &str)> = matches
        .iter()
        .map(|r| (r.0.as_str(), r.1.as_str()))
        .collect();
    keys.sort();
    keys.dedup();
    match keys.len() {
        1 => matches.iter().map(|r| PathBuf::from(&r.2)).collect(),
        _ => Vec::new(),
    }
}

/// The series folder name of chapter folder `dir` for a book title, unless it is a manga id: the
/// folder sits in a `<source>` folder named after one of `sources` (the download layout), or its
/// name is shaped like a provider's manga id wherever it sits ([`looks_like_manga_id`]).
pub fn series_folder_name(dir: &Path, sources: &[&str]) -> Option<String> {
    let dir = dir.canonicalize().ok()?;
    let series = dir.parent()?;
    let name = series.file_name()?.to_string_lossy().into_owned();
    let source = series.parent().and_then(Path::file_name);
    let in_source = source.is_some_and(|s| sources.iter().any(|known| s == *known));
    (!in_source && !looks_like_manga_id(&name)).then_some(name)
}

/// A provider manga id, not a title: a UUID (MangaDex), a ULID (Weeb Central), a
/// `<lang>-webtoon|canvas-<number>` (Webtoons), or a number of 5+ digits (MangaDot, Tapas;
/// shorter numbers can be titles, like "86").
pub fn looks_like_manga_id(name: &str) -> bool {
    let ulid = name.len() == 26
        && name
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase());
    let number = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let webtoon = match name.rsplitn(3, '-').collect::<Vec<_>>().as_slice() {
        [n, kind, lang] => number(n) && matches!(*kind, "webtoon" | "canvas") && !lang.is_empty(),
        _ => false,
    };
    uuid::Uuid::try_parse(name).is_ok() || ulid || webtoon || (number(name) && name.len() >= 5)
}
