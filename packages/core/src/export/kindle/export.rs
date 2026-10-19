//! Chapter directories -> device pages -> one file in the requested [`Format`]. Long strips go
//! through the smart cutter (stitched across files); regular pages are scaled to fit.

use super::cover::cover_page;
use super::input::{book_id, chapter_title};
use super::request::{ExportRequest, ExportStats, Progress};
use super::run::{Run, RunSpec};
use crate::domain::{Error, Result};
use crate::export::epub::ChapterEntry;
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
    let exporter = req.format.exporter();
    run.stats.output = exporter.export(run.pages, &meta, &dev, &req.output, req.replace)?;
    Ok(run.stats)
}

fn fallback_identity(req: &ExportRequest) -> String {
    let dirs: Vec<String> = req
        .chapter_dirs
        .iter()
        .map(|d| d.display().to_string())
        .collect();
    format!("{}|{}", req.title, dirs.join("|"))
}
