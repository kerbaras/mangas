//! Narrowing a chapter list to one scanlation group: sites like MangaDot list the same chapter
//! once per group, and a reader usually follows one of them.

use super::{parse_number, Availability, Chapter};

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

/// One release per chapter for when no group is picked (see [`same_slot`]): a downloadable
/// release beats an external one, then the earliest of `preferred` groups (case-insensitive),
/// then whichever came first. Keeps the list order; chapters without a number are all kept.
pub fn collapse_groups(chapters: Vec<Chapter>, preferred: &[String]) -> Vec<Chapter> {
    // Each slot remembers the first volume any of its releases named, so a release without a
    // volume never bridges two volumes.
    let mut slots: Vec<(Chapter, Option<String>)> = Vec::new();
    for chapter in chapters {
        let volume = volume_of(&chapter).map(String::from);
        let slot = slots.iter().position(|(kept, v)| {
            let compatible = match (v.as_deref(), volume.as_deref()) {
                (Some(x), Some(y)) => x == y,
                _ => true,
            };
            compatible
                && same_slot(
                    kept,
                    &Chapter {
                        volume: None,
                        ..chapter.clone()
                    },
                )
        });
        match slot {
            Some(i) => {
                if slots[i].1.is_none() {
                    slots[i].1 = volume;
                }
                if rank(&chapter, preferred) < rank(&slots[i].0, preferred) {
                    slots[i].0 = chapter;
                }
            }
            None => slots.push((chapter, volume)),
        }
    }
    slots.into_iter().map(|(c, _)| c).collect()
}

/// Whether `chapter` adds nothing to `stored`: another release of a stored chapter, unless it
/// is the first downloadable one.
pub fn already_have(stored: &[Chapter], chapter: &Chapter) -> bool {
    stored
        .iter()
        .any(|s| same_slot(s, chapter) && (s.is_available() || !chapter.is_available()))
}

/// Whether two chapters are releases of the same chapter: same language, same number (`1` is
/// `1.0`) and no conflicting volumes (numbering can restart per volume). Chapters without a
/// number never match.
pub fn same_slot(a: &Chapter, b: &Chapter) -> bool {
    let number = |c: &Chapter| {
        let n = c.number.trim();
        (!n.is_empty()).then(|| parse_number(n).map_or_else(|| n.to_string(), |v| v.to_string()))
    };
    let compatible = match (volume_of(a), volume_of(b)) {
        (Some(x), Some(y)) => x == y,
        _ => true,
    };
    a.language == b.language && compatible && number(a).is_some_and(|n| Some(n) == number(b))
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

fn volume_of(c: &Chapter) -> Option<&str> {
    c.volume.as_deref().map(str::trim).filter(|v| !v.is_empty())
}
