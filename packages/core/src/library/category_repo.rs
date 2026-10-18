use std::collections::HashSet;

use rusqlite::{params, ErrorCode, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::Library;
use crate::domain::{Error, MangaId, Result};

/// A user-made shelf. A title can sit on any number of categories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
}

const MAX_NAME: usize = 40;

/// Trims and collapses inner whitespace; empty or overly long names are refused.
pub fn clean_category_name(name: &str) -> Result<String> {
    let clean = name.split_whitespace().collect::<Vec<_>>().join(" ");
    match clean.chars().count() {
        0 => Err(Error::Other("a category needs a name".into())),
        n if n > MAX_NAME => Err(Error::Other(format!(
            "category names are at most {MAX_NAME} characters"
        ))),
        _ => Ok(clean),
    }
}

/// Categories in the order they were created.
pub fn list_categories(lib: &Library) -> Result<Vec<Category>> {
    let conn = lib.conn()?;
    let mut stmt = conn.prepare("SELECT id, name FROM category ORDER BY position, id")?;
    let rows = stmt.query_map([], |r| {
        Ok(Category {
            id: r.get(0)?,
            name: r.get(1)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Adds a category at the end. Names are unique regardless of case.
pub fn create_category(lib: &Library, name: &str) -> Result<Category> {
    let name = clean_category_name(name)?;
    let conn = lib.conn()?;
    conn.execute(
        "INSERT INTO category (name, position)
         VALUES (?1, (SELECT COALESCE(MAX(position), 0) + 1 FROM category))",
        params![name],
    )
    .map_err(|e| duplicate_or(e, &name))?;
    Ok(Category {
        id: conn.last_insert_rowid(),
        name,
    })
}

pub fn rename_category(lib: &Library, id: i64, name: &str) -> Result<Category> {
    let name = clean_category_name(name)?;
    let changed = lib
        .conn()?
        .execute(
            "UPDATE category SET name = ?2 WHERE id = ?1",
            params![id, name],
        )
        .map_err(|e| duplicate_or(e, &name))?;
    match changed {
        0 => Err(missing(id)),
        _ => Ok(Category { id, name }),
    }
}

/// Deletes a category; the titles on it stay in the library.
pub fn delete_category(lib: &Library, id: i64) -> Result<()> {
    match lib
        .conn()?
        .execute("DELETE FROM category WHERE id = ?1", params![id])?
    {
        0 => Err(missing(id)),
        _ => Ok(()),
    }
}

/// Replaces the categories of a followed title with `ids` (empty clears them).
pub fn set_manga_categories(lib: &Library, source: &str, id: &MangaId, ids: &[i64]) -> Result<()> {
    let mut conn = lib.conn()?;
    let tx = conn.transaction()?;
    let followed = tx
        .query_row(
            "SELECT 1 FROM manga WHERE source = ?1 AND id = ?2",
            params![source, id.0],
            |_| Ok(()),
        )
        .optional()?;
    if followed.is_none() {
        return Err(Error::NotFound(format!(
            "{source} manga {id} is not followed"
        )));
    }
    tx.execute(
        "DELETE FROM manga_category WHERE source = ?1 AND manga_id = ?2",
        params![source, id.0],
    )?;
    let known: HashSet<i64> = {
        let mut stmt = tx.prepare("SELECT id FROM category")?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        rows.collect::<std::result::Result<_, _>>()?
    };
    for category in ids {
        if !known.contains(category) {
            return Err(missing(*category));
        }
        tx.execute(
            "INSERT OR IGNORE INTO manga_category (source, manga_id, category_id)
             VALUES (?1, ?2, ?3)",
            params![source, id.0, category],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn missing(id: i64) -> Error {
    Error::NotFound(format!("category {id}"))
}

fn duplicate_or(e: rusqlite::Error, name: &str) -> Error {
    match e.sqlite_error_code() {
        Some(ErrorCode::ConstraintViolation) => {
            Error::Other(format!("a category named \"{name}\" already exists"))
        }
        _ => e.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_collapsed_and_bounded() {
        assert_eq!(clean_category_name("  To   read ").unwrap(), "To read");
        assert!(clean_category_name("   ").is_err());
        assert!(clean_category_name(&"x".repeat(41)).is_err());
        assert!(clean_category_name(&"é".repeat(40)).is_ok());
    }
}
