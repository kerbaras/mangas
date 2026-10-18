use rusqlite::{params, Connection};

use super::{CategoryState, ChapterState, LibrarySnapshot, TitleState};
use crate::domain::{Manga, Result};
use crate::library::manga_repo::upsert_on;
use crate::library::Library;

/// Restores a backup in one transaction. `add` are the titles to follow, each with the state to
/// give it. For every title in `snapshot` that ends up followed (new or already there), its
/// category assignments, synced languages and chapters are added; nothing already stored is
/// removed or overwritten, except that a chapter without an exported mark takes the backup's.
/// Categories match by name (ignoring case); new ones go after the existing ones.
pub fn restore_snapshot(
    lib: &Library,
    add: &[(Manga, TitleState)],
    snapshot: &LibrarySnapshot,
) -> Result<()> {
    let mut conn = lib.conn()?;
    let tx = conn.transaction()?;
    insert_categories(&tx, &snapshot.categories)?;
    for (manga, state) in add {
        upsert_on(&tx, manga)?;
        set_title_state(&tx, state)?;
    }
    for title in &snapshot.titles {
        link_title(&tx, title)?;
    }
    insert_chapters(&tx, &snapshot.chapters)?;
    tx.commit()?;
    Ok(())
}

fn insert_categories(conn: &Connection, categories: &[CategoryState]) -> Result<()> {
    let offset: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM category",
        [],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(
        "INSERT INTO category (name, position) VALUES (?1, ?2) ON CONFLICT (name) DO NOTHING",
    )?;
    for c in categories.iter().filter(|c| !c.name.trim().is_empty()) {
        stmt.execute(params![c.name, c.position + offset])?;
    }
    Ok(())
}

/// Missing times keep what the insert set (`added_at` defaults to now).
fn set_title_state(conn: &Connection, t: &TitleState) -> Result<()> {
    conn.execute(
        "UPDATE manga SET added_at = COALESCE(?3, added_at), updated_at = COALESCE(?4, updated_at),
                reading_status = ?5, auto_download = ?6,
                last_viewed_at = COALESCE(?7, last_viewed_at)
         WHERE source = ?1 AND id = ?2",
        params![
            t.source,
            t.id,
            t.added_at,
            t.updated_at,
            t.reading_status.as_str(),
            t.auto_download,
            t.last_viewed_at,
        ],
    )?;
    Ok(())
}

fn link_title(conn: &Connection, t: &TitleState) -> Result<()> {
    let mut category = conn.prepare(
        "INSERT OR IGNORE INTO manga_category (source, manga_id, category_id)
         SELECT m.source, m.id, c.id FROM manga m, category c
         WHERE m.source = ?1 AND m.id = ?2 AND c.name = ?3",
    )?;
    for name in &t.categories {
        category.execute(params![t.source, t.id, name])?;
    }
    let mut language = conn.prepare(
        "INSERT OR IGNORE INTO manga_language_sync (source, manga_id, language)
         SELECT source, id, ?3 FROM manga WHERE source = ?1 AND id = ?2",
    )?;
    for lang in &t.synced_languages {
        language.execute(params![t.source, t.id, lang])?;
    }
    Ok(())
}

fn insert_chapters(conn: &Connection, chapters: &[ChapterState]) -> Result<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO chapter
            (source, manga_id, id, title, language, volume, number, scanlation_group,
             availability, external_url, exported_at, first_seen_at)
         SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12
         WHERE EXISTS (SELECT 1 FROM manga WHERE source = ?1 AND id = ?2)
         ON CONFLICT (source, id) DO UPDATE SET
            exported_at = COALESCE(chapter.exported_at, excluded.exported_at)",
    )?;
    for c in chapters {
        stmt.execute(params![
            c.source,
            c.manga_id,
            c.id,
            c.title,
            c.language,
            c.volume,
            c.number,
            c.group,
            c.availability,
            c.external_url,
            c.exported_at,
            c.first_seen_at,
        ])?;
    }
    Ok(())
}
