use std::collections::HashMap;

use rusqlite::Connection;

use super::{CategoryState, ChapterState, LibrarySnapshot, TitleState};
use crate::domain::Result;
use crate::library::{Library, ReadingStatus};

type Key = (String, String);

/// Everything in [`LibrarySnapshot`], titles in the order they were followed.
pub fn read_snapshot(lib: &Library) -> Result<LibrarySnapshot> {
    let conn = lib.conn()?;
    Ok(LibrarySnapshot {
        categories: categories(&conn)?,
        titles: titles(&conn)?,
        chapters: chapters(&conn)?,
    })
}

fn categories(conn: &Connection) -> Result<Vec<CategoryState>> {
    let mut stmt = conn.prepare("SELECT name, position FROM category ORDER BY position, id")?;
    let rows = stmt.query_map([], |r| {
        Ok(CategoryState {
            name: r.get(0)?,
            position: r.get(1)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

fn grouped(conn: &Connection, sql: &str) -> Result<HashMap<Key, Vec<String>>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |r| Ok(((r.get(0)?, r.get(1)?), r.get(2)?)))?;
    let mut map: HashMap<Key, Vec<String>> = HashMap::new();
    for row in rows {
        let (key, value) = row?;
        map.entry(key).or_default().push(value);
    }
    Ok(map)
}

fn titles(conn: &Connection) -> Result<Vec<TitleState>> {
    let mut names = grouped(
        conn,
        "SELECT mc.source, mc.manga_id, c.name FROM manga_category mc
         JOIN category c ON c.id = mc.category_id ORDER BY c.position, c.id",
    )?;
    let mut languages = grouped(
        conn,
        "SELECT source, manga_id, language FROM manga_language_sync ORDER BY language",
    )?;
    let mut stmt = conn.prepare(
        "SELECT source, id, added_at, updated_at, reading_status, auto_download, last_viewed_at
         FROM manga ORDER BY rowid",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(TitleState {
            source: r.get(0)?,
            id: r.get(1)?,
            added_at: r.get(2)?,
            updated_at: r.get(3)?,
            reading_status: ReadingStatus::parse(&r.get::<_, String>(4)?),
            auto_download: r.get(5)?,
            last_viewed_at: r.get(6)?,
            ..Default::default()
        })
    })?;
    let mut titles = Vec::new();
    for row in rows {
        let mut title = row?;
        let key = (title.source.clone(), title.id.clone());
        title.categories = names.remove(&key).unwrap_or_default();
        title.synced_languages = languages.remove(&key).unwrap_or_default();
        titles.push(title);
    }
    Ok(titles)
}

fn chapters(conn: &Connection) -> Result<Vec<ChapterState>> {
    let mut stmt = conn.prepare(
        "SELECT source, manga_id, id, title, language, volume, number, scanlation_group,
                availability, external_url, exported_at, first_seen_at
         FROM chapter ORDER BY rowid",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(ChapterState {
            source: r.get(0)?,
            manga_id: r.get(1)?,
            id: r.get(2)?,
            title: r.get(3)?,
            language: r.get(4)?,
            volume: r.get(5)?,
            number: r.get(6)?,
            group: r.get(7)?,
            availability: r.get(8)?,
            external_url: r.get(9)?,
            exported_at: r.get(10)?,
            first_seen_at: r.get(11)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}
