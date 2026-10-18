//! What an export of a manga will produce, before any image is read.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{book_meta, file_name, group, render, Grouping, Volume, DEFAULT_TEMPLATE};
use crate::domain::{parse_number, Chapter, Manga};
use crate::export::kindle::{Device, ExportRequest};
use crate::smartcut::Options;

/// One EPUB to write: its title, file name and downloaded chapters in reading order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VolumePlan {
    pub number: Option<String>,
    pub title: String,
    pub file_name: String,
    pub chapters: Vec<Chapter>,
}

/// Books for the downloaded chapters among `chapters` (in reading order). `template` defaults
/// to [`DEFAULT_TEMPLATE`]; `title` names a single book and overrides the template.
pub fn plan(
    manga: &Manga,
    chapters: &[Chapter],
    grouping: Grouping,
    template: Option<&str>,
    title: Option<&str>,
) -> Vec<VolumePlan> {
    let downloaded: Vec<Chapter> = chapters
        .iter()
        .filter(|c| c.is_downloaded())
        .cloned()
        .collect();
    let volumes = group(&downloaded, grouping);
    let single = volumes.len() == 1;
    let template = template
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or(DEFAULT_TEMPLATE);
    volumes
        .into_iter()
        .map(|v: Volume| {
            let title = match title.map(str::trim).filter(|t| !t.is_empty()) {
                Some(t) if single => t.to_string(),
                _ => render(template, &manga.title, &v),
            };
            VolumePlan {
                number: v.number,
                file_name: file_name(&title),
                title,
                chapters: v.chapters,
            }
        })
        .collect()
}

/// The planned book with volume number `number`: an exact label first, else a numeric match
/// ("3" matches "03" and "3.0").
pub fn find_volume<'a>(plans: &'a [VolumePlan], number: &str) -> Option<&'a VolumePlan> {
    let wanted = number.trim();
    plans.iter().find(|p| label(p) == Some(wanted)).or_else(|| {
        let target = parse_number(wanted)?;
        plans
            .iter()
            .find(|p| label(p).and_then(parse_number) == Some(target))
    })
}

fn label(p: &VolumePlan) -> Option<&str> {
    p.number.as_deref().map(str::trim)
}

/// The export of `plan` into `output_dir`, with `cover` (the series cover file) when known.
pub fn request(
    manga: &Manga,
    plan: &VolumePlan,
    device: Device,
    cut: Options,
    output_dir: &Path,
    cover: Option<PathBuf>,
) -> ExportRequest {
    let volume = Volume {
        number: plan.number.clone(),
        chapters: plan.chapters.clone(),
    };
    ExportRequest {
        chapter_dirs: plan
            .chapters
            .iter()
            .filter_map(|c| c.local_path.as_ref().map(PathBuf::from))
            .collect(),
        device,
        cut,
        title: plan.title.clone(),
        output: output_dir.join(&plan.file_name),
        meta: book_meta(manga, &volume, cover),
    }
}
