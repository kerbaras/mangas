//! What an export of a manga will produce, before any image is read.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::DEFAULT_TEMPLATE;
use super::{book_meta, file_name, group, render, series_file_name, Grouping, Volume};
use crate::domain::{parse_number, Chapter, Manga, Result};
use crate::export::kindle::{Device, ExportRequest};
use crate::export::{free_names_in, Format};
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
            let (title, file_name) = match title.map(str::trim).filter(|t| !t.is_empty()) {
                Some(t) if single => (t.to_string(), file_name(t)),
                _ => {
                    let title = render(template, &manga.title, &v);
                    let name = series_file_name(&title, &manga.title);
                    (title, name)
                }
            };
            VolumePlan {
                number: v.number,
                file_name,
                title,
                chapters: v.chapters,
            }
        })
        .collect()
}

/// `plans` ready to write with `replace`: then two books planned under one file name (a
/// template without `{volume}`) are numbered ` (2)`, ` (3)`… as the writer numbers them in an
/// empty folder, so no book of the run writes over another. Without `replace` the writer
/// already gives each book its own file.
pub fn one_file_each(plans: Vec<VolumePlan>, replace: bool) -> Vec<VolumePlan> {
    if !replace {
        return plans;
    }
    let mut used = HashSet::new();
    let mut free = |name: &str| {
        (1..)
            .map(|n| numbered(name, n))
            .find(|name| used.insert(name.to_lowercase()))
            .unwrap_or_default()
    };
    plans
        .into_iter()
        .map(|p| VolumePlan {
            file_name: free(&p.file_name),
            ..p
        })
        .collect()
}

/// The file names `plans` are written under in `dir` as `ext` files: the planned names when
/// replacing (see [`one_file_each`]), else the first free ` (n)` name for each, as the writer
/// picks them one book after another.
pub fn written_names(
    plans: &[VolumePlan],
    dir: &Path,
    ext: &str,
    replace: bool,
) -> Result<Vec<String>> {
    let wanted: Vec<String> = plans
        .iter()
        .map(|p| {
            let name = Path::new(&p.file_name).with_extension(ext);
            name.to_string_lossy().into_owned()
        })
        .collect();
    match replace {
        true => Ok(wanted),
        false => free_names_in(dir, &wanted),
    }
}

/// `name` for `n` = 1, else `stem (n).ext`.
fn numbered(name: &str, n: usize) -> String {
    let path = Path::new(name);
    match (n, path.file_stem(), path.extension()) {
        (1, _, _) => name.to_string(),
        (_, Some(stem), Some(ext)) => {
            format!("{} ({n}).{}", stem.to_string_lossy(), ext.to_string_lossy())
        }
        _ => format!("{name} ({n})"),
    }
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

/// The EPUB export of `plan` into `output_dir`, with `cover` (the series cover file) when
/// known; [`ExportRequest::with_format`] picks another format.
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
        split: None,
        chapter_dirs: plan
            .chapters
            .iter()
            .filter_map(|c| c.local_path.as_ref().map(PathBuf::from))
            .collect(),
        device,
        format: Format::Epub,
        cut,
        page: Default::default(),
        title: plan.title.clone(),
        title_explicit: false,
        output: output_dir.join(&plan.file_name),
        replace: false,
        meta: book_meta(manga, &volume, cover),
        labels: Default::default(),
    }
}
