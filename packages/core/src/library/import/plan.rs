//! Pure mapping from scanned entries to the manga and chapters stored in the library.

use std::collections::HashSet;

use super::names::{parse_chapter_name, short_hash, slug, strip_extension};
use super::scan::Entry;
use super::LOCAL_SOURCE;
use crate::domain::{compare_numbers, Chapter, ChapterId, Manga, MangaId, MangaStatus};

/// A chapter to import and where its pages come from.
#[derive(Debug, Clone)]
pub struct Planned {
    pub entry: Entry,
    pub chapter: Chapter,
}

/// The `local` manga for `title`; its id is the title's slug so re-imports land on it.
pub fn manga_for(title: &str) -> Manga {
    Manga {
        id: MangaId(slug(title)),
        source: LOCAL_SOURCE.to_string(),
        title: title.to_string(),
        description: String::new(),
        cover_url: None,
        status: MangaStatus::Unknown,
        ..Default::default()
    }
}

/// Ids to try for a new series, in order: the slug, `<slug>-<hash of title>`, then numbered
/// variants, for when another title already owns the slug (`A B` vs `A! B`).
pub fn candidate_ids(slug: &str, title: &str) -> Vec<MangaId> {
    let hash = short_hash(title);
    let hashed = format!("{slug}-{}", &hash[..6.min(hash.len())]);
    let numbered = (2..50).map(|n| format!("{hashed}-{n}"));
    [slug.to_string(), hashed.clone()]
        .into_iter()
        .chain(numbered)
        .map(MangaId)
        .collect()
}

/// One chapter per entry. Ids hash the manga id and the entry name, so importing the same
/// folder again yields the same ids; numbers and volumes come from the names. Sorted by
/// chapter number; unnumbered entries keep their (natural) name order at the end.
pub fn plan(manga_id: &MangaId, language: &str, entries: Vec<Entry>) -> Vec<Planned> {
    let mut seen = HashSet::new();
    let mut planned: Vec<Planned> = entries
        .into_iter()
        .map(|entry| {
            let id = unique_id(&mut seen, &manga_id.0, &entry.name);
            let parsed = parse_chapter_name(&entry.name);
            let chapter = Chapter {
                id: ChapterId(id),
                manga_id: manga_id.clone(),
                title: Some(strip_extension(&entry.name).to_string()),
                language: language.to_string(),
                volume: parsed.volume,
                number: parsed.number.unwrap_or_default(),
                local_path: None,
                group: None,
                availability: Default::default(),
                exported_at: None,
            };
            Planned { entry, chapter }
        })
        .collect();
    planned.sort_by(|a, b| compare_numbers(&a.chapter.number, &b.chapter.number));
    planned
}

fn unique_id(seen: &mut HashSet<String>, manga: &str, name: &str) -> String {
    let mut n = 0;
    loop {
        let key = if n == 0 {
            format!("{manga}\0{name}")
        } else {
            format!("{manga}\0{name}\0{n}")
        };
        let id = short_hash(&key);
        if seen.insert(id.clone()) {
            return id;
        }
        n += 1;
    }
}
