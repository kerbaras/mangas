//! Per-series export presets (`export_preset`), and finding the series of chapter folders.

use std::path::Path;

use rusqlite::{params, OptionalExtension};

use super::Library;
use crate::domain::{Error, MangaId, ReadingDirection, Result};
use crate::export::ExportPreset;

/// The preset saved for `source`/`manga_id`. A row that no longer parses is ignored (logged),
/// so an export never fails over a stale preset.
pub fn get_export_preset(
    lib: &Library,
    source: &str,
    manga_id: &str,
) -> Result<Option<ExportPreset>> {
    let conn = lib.conn()?;
    let json: Option<String> = conn
        .query_row(
            "SELECT preset FROM export_preset WHERE source = ?1 AND manga_id = ?2",
            params![source, manga_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(json.and_then(|j| match serde_json::from_str(&j) {
        Ok(p) => Some(p),
        Err(e) => {
            tracing::warn!(source, manga_id, error = %e, "ignoring unreadable export preset");
            None
        }
    }))
}

/// Saves `preset` for the series (validated first); an empty preset clears it.
pub fn set_export_preset(
    lib: &Library,
    source: &str,
    manga_id: &str,
    preset: &ExportPreset,
) -> Result<()> {
    if preset.is_empty() {
        clear_export_preset(lib, source, manga_id)?;
        return Ok(());
    }
    preset.validate()?;
    let json = serde_json::to_string(preset).map_err(|e| Error::Other(e.to_string()))?;
    let conn = lib.conn()?;
    let known: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM manga WHERE source = ?1 AND id = ?2)",
        params![source, manga_id],
        |r| r.get(0),
    )?;
    if !known {
        return Err(Error::Other(format!(
            "{source}/{manga_id} is not in the library"
        )));
    }
    conn.execute(
        "INSERT INTO export_preset (source, manga_id, preset) VALUES (?1, ?2, ?3)
         ON CONFLICT (source, manga_id)
         DO UPDATE SET preset = excluded.preset, updated_at = datetime('now')",
        params![source, manga_id, json],
    )?;
    Ok(())
}

/// Removes the series preset; `false` when there was none.
pub fn clear_export_preset(lib: &Library, source: &str, manga_id: &str) -> Result<bool> {
    let n = lib.conn()?.execute(
        "DELETE FROM export_preset WHERE source = ?1 AND manga_id = ?2",
        params![source, manga_id],
    )?;
    Ok(n > 0)
}

/// The series (`source`, `manga_id`) every folder in `dirs` is a downloaded chapter of; `None`
/// when a folder is unknown or they span several series.
pub fn series_of_dirs(lib: &Library, dirs: &[&Path]) -> Result<Option<(String, String)>> {
    let each: Vec<Vec<&Path>> = dirs.iter().map(|d| vec![*d]).collect();
    series_of_spelled(lib, &each)
}

/// The reading direction the source gives `source`/`manga_id`; none when it isn't followed or
/// its source doesn't say.
pub fn source_direction(
    lib: &Library,
    source: &str,
    manga_id: &str,
) -> Result<Option<ReadingDirection>> {
    let manga = super::get_manga(lib, source, &MangaId(manga_id.to_string()))?;
    Ok(manga.and_then(|m| m.direction))
}

/// [`preset_for_dirs`] over what the series' source says (its reading direction): the layer
/// between the explicit choices and the defaults when exporting chapter folders.
pub fn series_layer_for_dirs(lib: &Library, dirs: &[&Path]) -> Result<Option<ExportPreset>> {
    let Some((source, id)) = series_of_dirs(lib, dirs)? else {
        return Ok(None);
    };
    let below = ExportPreset {
        direction: source_direction(lib, &source, &id)?,
        ..Default::default()
    };
    let preset = get_export_preset(lib, &source, &id)?.unwrap_or_default();
    Ok(Some(preset.over(&below)))
}

/// [`series_of_dirs`] with each folder in several spellings (as typed, absolute, resolved):
/// one stored spelling is enough.
pub fn series_of_spelled(lib: &Library, dirs: &[Vec<&Path>]) -> Result<Option<(String, String)>> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare(
        "SELECT DISTINCT source, manga_id FROM chapter
         WHERE local_path IS NOT NULL AND rtrim(local_path, '/\\') = ?1",
    )?;
    let mut series: Option<(String, String)> = None;
    for spellings in dirs {
        let mut found: Vec<(String, String)> = Vec::new();
        for dir in spellings {
            let path = dir.to_string_lossy();
            let rows = stmt.query_map(params![path.trim_end_matches(['/', '\\'])], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
            found.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
        }
        found.sort();
        found.dedup();
        match (found.as_slice(), &series) {
            ([one], None) => series = Some(one.clone()),
            ([one], Some(s)) if one == s => {}
            _ => return Ok(None),
        }
    }
    Ok(series)
}

/// The series of `dirs` and its saved preset, when they belong to one series that has one.
pub fn preset_for_dirs(lib: &Library, dirs: &[&Path]) -> Result<Option<ExportPreset>> {
    match series_of_dirs(lib, dirs)? {
        Some((source, manga_id)) => get_export_preset(lib, &source, &manga_id),
        None => Ok(None),
    }
}
