//! Splitting an ordered chapter list into volumes.

use super::{Grouping, Volume};
use crate::domain::{compare_numbers, Chapter};

fn volume_of(c: &Chapter) -> Option<String> {
    c.volume
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// Groups `chapters` (already in reading order) into volumes. Empty input gives no volumes.
pub fn group(chapters: &[Chapter], grouping: Grouping) -> Vec<Volume> {
    if chapters.is_empty() {
        return Vec::new();
    }
    match grouping {
        Grouping::Single => vec![Volume {
            number: single_number(chapters),
            chapters: chapters.to_vec(),
        }],
        Grouping::Every(size) => chapters
            .chunks(size.max(1))
            .enumerate()
            .map(|(i, chunk)| Volume {
                number: Some((i + 1).to_string()),
                chapters: chunk.to_vec(),
            })
            .collect(),
        Grouping::ByVolume => by_volume(chapters),
        Grouping::PerChapter => chapters
            .iter()
            .map(|c| Volume {
                number: None,
                chapters: vec![c.clone()],
            })
            .collect(),
    }
}

/// The provider volume shared by every chapter of a single book, if there is one.
fn single_number(chapters: &[Chapter]) -> Option<String> {
    let first = volume_of(chapters.first()?)?;
    chapters
        .iter()
        .all(|c| volume_of(c).as_deref() == Some(first.as_str()))
        .then_some(first)
}

fn by_volume(chapters: &[Chapter]) -> Vec<Volume> {
    let mut volumes: Vec<Volume> = Vec::new();
    let mut loose: Vec<Chapter> = Vec::new();
    for c in chapters {
        let Some(number) = volume_of(c) else {
            loose.push(c.clone());
            continue;
        };
        match volumes
            .iter_mut()
            .find(|v| v.number.as_deref() == Some(number.as_str()))
        {
            Some(v) => v.chapters.push(c.clone()),
            None => volumes.push(Volume {
                number: Some(number),
                chapters: vec![c.clone()],
            }),
        }
    }
    volumes.sort_by(|a, b| {
        compare_numbers(
            a.number.as_deref().unwrap_or_default(),
            b.number.as_deref().unwrap_or_default(),
        )
    });
    if !loose.is_empty() {
        volumes.push(Volume {
            number: None,
            chapters: loose,
        });
    }
    volumes
}
