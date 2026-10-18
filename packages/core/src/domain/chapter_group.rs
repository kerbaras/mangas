//! Narrowing a chapter list to one scanlation group: sites like MangaDot list the same chapter
//! once per group, and a reader usually follows one of them.

use std::collections::HashMap;

use super::{Availability, Chapter};

/// A group in a chapter list and how many of its chapters the list holds.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GroupCount {
    pub name: String,
    pub chapters: usize,
}

/// The groups credited in `chapters`, most chapters first, ties by name. Chapters without a
/// group are not counted: there is nothing to narrow them to.
pub fn groups_of(chapters: &[Chapter]) -> Vec<GroupCount> {
    let mut counts: Vec<GroupCount> = Vec::new();
    for name in chapters.iter().filter_map(|c| c.group.as_deref()) {
        match counts.iter_mut().find(|g| g.name == name) {
            Some(g) => g.chapters += 1,
            None => counts.push(GroupCount {
                name: name.to_string(),
                chapters: 1,
            }),
        }
    }
    counts.sort_by(|a, b| b.chapters.cmp(&a.chapters).then(a.name.cmp(&b.name)));
    counts
}

/// The chapters credited to `group` (the whole label, so "A, B" is its own group); every
/// chapter when `group` is `None`.
pub fn in_group(chapters: Vec<Chapter>, group: Option<&str>) -> Vec<Chapter> {
    match group {
        None => chapters,
        Some(g) => chapters
            .into_iter()
            .filter(|c| c.group.as_deref() == Some(g))
            .collect(),
    }
}

/// A user-typed group name: trimmed, empty meaning every group.
pub fn normalize_group(name: Option<&str>) -> Option<String> {
    name.map(str::trim)
        .filter(|n| !n.is_empty())
        .map(String::from)
}

/// One release per chapter number and language, for when no group is picked: a downloadable
/// release beats an external one, then the earliest of `preferred` groups (case-insensitive),
/// then whichever came first. Keeps the list order; chapters without a number are all kept.
pub fn collapse_groups(chapters: Vec<Chapter>, preferred: &[String]) -> Vec<Chapter> {
    let mut out: Vec<Chapter> = Vec::new();
    let mut slots: HashMap<(String, String), usize> = HashMap::new();
    for chapter in chapters {
        let number = chapter.number.trim().to_string();
        if number.is_empty() {
            out.push(chapter);
            continue;
        }
        match slots.get(&(chapter.language.clone(), number.clone())) {
            Some(&i) if rank(&chapter, preferred) < rank(&out[i], preferred) => out[i] = chapter,
            Some(_) => {}
            None => {
                slots.insert((chapter.language.clone(), number), out.len());
                out.push(chapter);
            }
        }
    }
    out
}

fn rank(chapter: &Chapter, preferred: &[String]) -> (u8, usize) {
    let tier = match &chapter.availability {
        Availability::Available => 0,
        Availability::External { url: Some(_) } => 1,
        _ => 2,
    };
    let names: Vec<&str> = chapter
        .group
        .as_deref()
        .map_or_else(Vec::new, |g| g.split(", ").map(str::trim).collect());
    let group = preferred
        .iter()
        .position(|p| names.iter().any(|n| n.eq_ignore_ascii_case(p.trim())))
        .unwrap_or(usize::MAX);
    (tier, group)
}
