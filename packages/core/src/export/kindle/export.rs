//! Chapter directories -> device pages -> one file in the requested [`Format`]. Long strips go
//! through the smart cutter (stitched across files); regular pages are scaled to fit.

use super::cover::cover_page;
use super::input::{book_id, chapter_title};
use super::parts::{
    chapters_in, free_book_path, oversized, part_path, part_title, split_points, staging,
};
use super::request::{ExportRequest, ExportStats, Part, Progress};
use super::run::{Run, RunSpec};
use super::stale::{check_export_folder, folder_of, remove_stale};
use crate::domain::{io_at, placed_at};
use crate::domain::{Error, Result};
use crate::export::epub::ChapterEntry;
use crate::export::{Meta, PageSpool};
use crate::smartcut::RowPenalizer;
use std::ops::Range;
use std::path::Path;

/// Runs the export. `penalizer` adds detector-based cut costs; `progress` receives events.
pub fn export(
    req: &ExportRequest,
    penalizer: Option<RowPenalizer>,
    progress: &mut dyn FnMut(Progress),
) -> Result<ExportStats> {
    if req.chapter_dirs.is_empty() {
        return Err(Error::Other("export: no chapter directories".into()));
    }
    check_export_folder(folder_of(&req.output), req.split.is_some(), req.replace)?;
    let output = match req.replace {
        true => req.output.clone(),
        false => free_book_path(&req.output)?,
    };
    let req = &ExportRequest {
        output,
        ..req.clone()
    };
    let dev = req.device;
    let spec = RunSpec {
        device: dev,
        cut: req.cut,
        page: req.page,
        penalizer,
        limit: None,
        cancelled: &|| false,
        spool_dir: Some(folder_of(&req.output)),
    };
    let mut run = Run::new(spec, progress)?;
    let mut chapters = Vec::new();
    let total = req.chapter_dirs.len();
    for (index, dir) in req.chapter_dirs.iter().enumerate() {
        (run.progress)(Progress::Chapter {
            index,
            total,
            path: dir.clone(),
        });
        let first_page = run.pages.len();
        run.chapter(dir).map_err(|e| match e {
            Error::NoSpace { .. } => e,
            e @ Error::Io { .. } => placed_at(e, dir),
            e => Error::Other(format!("{}: {e}", dir.display())),
        })?;
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
        authors: src.authors.clone(),
        language: src.language.clone().unwrap_or_else(|| "en".into()),
        series: src.series.clone(),
        right_to_left: src.direction.is_rtl(),
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
        .map(|max| split_points(run.pages.info(), meta.cover.as_ref(), max, req.format))
        .filter(|r| r.len() > 1);
    let exporter = req.format.exporter();
    match ranges {
        None => {
            run.stats.output = exporter.export(run.pages, &meta, &dev, &req.output, req.replace)?
        }
        Some(ranges) => {
            run.stats.parts = write_parts(req, &run.pages, &meta, &ranges)?;
            run.stats.output = run.stats.parts[0].path.clone();
        }
    }
    if req.replace {
        remove_stale(&req.output, run.stats.parts.len())?;
    }
    run.stats.oversized = oversized(&run.stats.outputs());
    run.stats.jpeg_quality = req.page.image_settings(&req.device).quality;
    Ok(run.stats)
}

/// Writes `pages` as one book per range, each copied into its own spool in the export folder
/// before it is written. A part that fails removes the parts written before
/// it: half a book is never left behind. Replacing parts, each is built under a staging name and
/// they are renamed over the old ones only once all are written, so a failure keeps the old set.
fn write_parts(
    req: &ExportRequest,
    pages: &PageSpool,
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
        let out = part_path(&req.output, n);
        let target = match req.replace {
            true => staging(&out),
            false => out,
        };
        let exported = pages
            .copy_range(range.clone(), Some(folder_of(&req.output)))
            .and_then(|pages| {
                let exporter = req.format.exporter();
                exporter.export(pages, &part, &req.device, &target, req.replace)
            });
        match exported {
            Ok(path) => written.push(Part {
                path,
                pages: range.len(),
                chapters: part.chapters.len(),
            }),
            Err(e) => {
                for part in &written {
                    let _ = std::fs::remove_file(&part.path);
                }
                return Err(e);
            }
        }
    }
    match req.replace {
        true => publish(written, &req.output),
        false => Ok(written),
    }
}

/// Renames the staged parts over their final names, in order.
fn publish(staged: Vec<Part>, output: &Path) -> Result<Vec<Part>> {
    let mut done = Vec::with_capacity(staged.len());
    for (i, part) in staged.iter().enumerate() {
        let path = part_path(output, i + 1);
        if let Err(e) = std::fs::rename(&part.path, &path) {
            for left in &staged[i..] {
                let _ = std::fs::remove_file(&left.path);
            }
            return Err(io_at(&path)(e));
        }
        done.push(Part {
            path,
            ..part.clone()
        });
    }
    Ok(done)
}

fn fallback_identity(req: &ExportRequest) -> String {
    let dirs: Vec<String> = req
        .chapter_dirs
        .iter()
        .map(|d| d.display().to_string())
        .collect();
    format!("{}|{}", req.title, dirs.join("|"))
}
