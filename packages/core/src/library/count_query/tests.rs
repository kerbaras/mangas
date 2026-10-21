//! The aggregate query against the row-by-row count it replaced, on a varied library.

use std::collections::HashMap;

use rusqlite::{params, Connection};

use super::{counts_by_manga, Key};
use crate::library::counts::{count_chapters, ChapterCounts, ChapterMark};
use crate::library::open_in_memory;

/// The pre-aggregate path: every chapter row loaded as a mark, counted in Rust.
fn counted_row_by_row(conn: &Connection) -> HashMap<Key, ChapterCounts> {
    let mut stmt = conn
        .prepare(
            "SELECT c.source, c.manga_id, c.number, c.availability = 'available',
                    c.local_path IS NOT NULL, c.exported_at IS NOT NULL, c.id,
                    c.first_seen_at IS NOT NULL
                      AND c.first_seen_at > COALESCE(m.last_viewed_at, '')
             FROM chapter c JOIN manga m ON m.source = c.source AND m.id = c.manga_id
             WHERE m.chapter_group IS NULL OR c.scanlation_group = m.chapter_group",
        )
        .unwrap();
    let mut marks: HashMap<Key, Vec<ChapterMark>> = HashMap::new();
    let rows = stmt
        .query_map([], |r| {
            let mark = ChapterMark {
                id: r.get(6)?,
                number: r.get(2)?,
                available: r.get(3)?,
                downloaded: r.get(4)?,
                exported: r.get(5)?,
                new: r.get(7)?,
            };
            Ok(((r.get(0)?, r.get(1)?), mark))
        })
        .unwrap();
    for row in rows {
        let (key, mark) = row.unwrap();
        marks.entry(key).or_default().push(mark);
    }
    marks
        .into_iter()
        .map(|(k, m)| (k, count_chapters(&m)))
        .collect()
}

const NUMBERS: [&str; 17] = [
    "1", "1.0", "01", " 2 ", "2.5", "2.50", "10", "1e1", "Oneshot", "", "  ", "inf", "NaN", "3",
    "0", "-0", "-0.0",
];
const AVAILABILITY: [&str; 3] = ["available", "locked", "external"];
const GROUPS: [Option<&str>; 3] = [Some("A"), Some("B"), None];

/// A small linear congruential generator: varied but repeatable rows.
struct Lcg(u64);

impl Lcg {
    fn pick(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) % n as u64) as usize
    }
}

fn seed(conn: &Connection) {
    let mut rng = Lcg(7);
    for t in 0..40 {
        let (source, id) = (["mangadex", "local", "webtoons"][t % 3], format!("m{t}"));
        let group = [None, Some("A"), Some("Z")][rng.pick(3)];
        let viewed = [None, Some("2026-01-02 00:00:00.000")][rng.pick(2)];
        conn.execute(
            "INSERT INTO manga (id, source, title, chapter_group, last_viewed_at)
             VALUES (?1, ?2, ?1, ?3, ?4)",
            params![id, source, group, viewed],
        )
        .unwrap();
        // Titles 0 and 1 have no chapters at all.
        for c in 0..if t < 2 { 0 } else { rng.pick(60) } {
            let local = [None, Some("/x")][rng.pick(2)];
            let exported = [None, Some("2026-01-01T00:00:00Z")][rng.pick(3).min(1)];
            let seen = [
                None,
                Some("2026-01-01 00:00:00.000"),
                Some("2026-01-03 00:00:00.000"),
            ][rng.pick(3)];
            conn.execute(
                "INSERT INTO chapter (id, manga_id, source, language, number, local_path,
                     scanlation_group, availability, exported_at, first_seen_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    format!("{id}-c{c}"),
                    id,
                    source,
                    ["en", "es"][rng.pick(2)],
                    NUMBERS[rng.pick(NUMBERS.len())],
                    local,
                    GROUPS[rng.pick(GROUPS.len())],
                    AVAILABILITY[rng.pick(AVAILABILITY.len())],
                    exported,
                    seen
                ],
            )
            .unwrap();
        }
    }
}

#[test]
fn aggregate_counts_match_the_row_by_row_counts() {
    let lib = open_in_memory().unwrap();
    let conn = lib.conn().unwrap();
    seed(&conn);
    let aggregate = counts_by_manga(&conn).unwrap();
    let expected = counted_row_by_row(&conn);
    assert!(
        expected.len() > 20,
        "{} titles with chapters",
        expected.len()
    );
    assert!(expected
        .values()
        .any(|c| c.downloaded > 0 && c.exported > 0 && c.new > 0));
    assert_eq!(aggregate, expected);
    assert!(!aggregate.contains_key(&("mangadex".into(), "m0".into())));
}
