//! Narrowing a chapter list to one scanlation group: sites like MangaDot list the same chapter
//! once per group, and a reader usually follows one of them.

use super::Chapter;

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
