//! Chapter directories -> device pages -> one file in the requested [`Format`]. Long strips go
//! through the smart cutter (stitched across files); regular pages are scaled to fit.

use super::cover::cover_page;
use std::ops::Range;
use super::input::{book_id, chapter_title};
use super::parts::{chapters_in, part_path, part_title, split_points};
use super::request::{ExportRequest, ExportStats, Part, Progress};
use super::run::{Run, RunSpec};
use crate::domain::{Error, Result};
use crate::export::epub::{ChapterEntry, PageImage};
use crate::export::Meta;
use crate::smartcut::RowPenalizer;

/// Runs the export. `penalizer` adds detector-based cut costs; `progress` receives events.
pub fn export(
    req: &ExportRequest,
    penalizer: Option<RowPenalizer>,
    progress: &mut dyn FnMut(Progress),
) -> Result<ExportStats> {
    if req.chapter_dirs.is_empty() {
        return Err(Error::Other("export: no chapter directories".into()));
    }
    let dev = req.device;
    let spec = RunSpec {
        device: dev,
        cut: req.cut,
        page: req.page,
        penalizer,
        limit: None,
        cancelled: &|| false,
    };
    let mut run = Run::new(spec, progress);
    let mut chapters = Vec::new();
    let total = req.chapter_dirs.len();
    for (index, dir) in req.chapter_dirs.iter().enumerate() {
        (run.progress)(Progress::Chapter {
            index,
            total,
            path: dir.clone(),
        });
        let first_page = run.pages.len();
        run.chapter(dir)
            .map_err(|e| Error::Other(format!("{}: {e}", dir.display())))?;
        if run.pages.len() > first_page {
            let title = req.meta.chapter_titles.get(index).cloned();
            chapters.push(ChapterEntry {
                title: title.unwrap_or_else(|| chapter_title(dir, index)),
                first_page,
            });
        }
    }
    (run.progress)(Progress::Writing {
        pages: run.pages.len(),
    });
    let src = &req.meta;
    let meta = Meta {
        id: book_id(
            &src.identity
                .clone()
                .unwrap_or_else(|| fallback_identity(req)),
        ),
        title: req.title.clone(),
        authors: if src.authors.is_empty() {
            vec!["mangas".into()]
        } else {
            src.authors.clone()
        },
        language: src.language.clone().unwrap_or_else(|| "en".into()),
        series: src.series.clone(),
        right_to_left: false,
        chapters,
        labels: req.labels.clone().or_series(src.series.as_ref()),
        title_explicit: req.title_explicit,
        cover: src
            .cover
            .as_deref()
            .and_then(|path| cover_page(path, &run.settings)),
    };
    let ranges = req
        .split
        .map(|max| split_points(&run.pages, meta.cover.as_ref(), max))
        .filter(|r| r.len() > 1);
    let exporter = req.format.exporter();
    match ranges {
        None => run.stats.output = exporter.export(run.pages, &meta, &dev, &req.output, req.replace)?,
        Some(ranges) => {
            run.stats.parts = write_parts(req, run.pages, &meta, &ranges)?;
            run.stats.output = run.stats.parts[0].path.clone();
        }
    }
    Ok(run.stats)
}

/// Writes `pages` as one book per range. A part that fails removes the parts written before
/// it: half a book is never left behind.
fn write_parts(
    req: &ExportRequest,
    mut pages: Vec<PageImage>,
    meta: &Meta,
    ranges: &[Range<usize>],
) -> Result<Vec<Part>> {
    let mut written = Vec::new();
    for (i, range) in ranges.iter().enumerate() {
        let n = i + 1;
        let part = Meta {
            id: book_id(&format!("{}|part {n}", meta.id)),
            title: part_title(&meta.title, n, ranges.len()),
            chapters: chapters_in(&meta.chapters, range),
            ..meta.clone()
        };
        let rest = pages.split_off(range.len());
        let out = part_path(&req.output, n);
        match req.format.exporter().export(pages, &part, &req.device, &out, req.replace) {
            Ok(path) => written.push(Part {
                path,
                pages: range.len(),
            }),
            Err(e) => {
                for part in &written {
                    let _ = std::fs::remove_file(&part.path);
                }
                return Err(e);
            }
        }
        pages = rest;
    }
    Ok(written)
}

fn fallback_identity(req: &ExportRequest) -> String {
    let dirs: Vec<String> = req
        .chapter_dirs
        .iter()
        .map(|d| d.display().to_string())
        .collect();
    format!("{}|{}", req.title, dirs.join("|"))
}
