//! Chapter directories -> device pages -> one file in the requested [`Format`]. Long strips go
//! through the smart cutter (stitched across files); regular pages are scaled to fit.

use std::path::{Path, PathBuf};
use std::time::Instant;

use image::RgbaImage;
use rayon::prelude::*;

use super::cover::cover_page;
use super::input::{book_id, chapter_files, chapter_title, decode, ChapterFiles};
use super::process::{process_page, ImageSettings};
use super::request::{ExportRequest, ExportStats, Progress};
use super::Device;
use crate::domain::{Error, Result};
use crate::export::epub::{ChapterEntry, PageImage};
use crate::export::Meta;
use crate::smartcut::{is_long_strip, RowPenalizer, StripPaginator};

struct Run<'a> {
    settings: ImageSettings,
    paginator: StripPaginator,
    pages: Vec<PageImage>,
    stats: ExportStats,
    progress: &'a mut dyn FnMut(Progress),
}

impl Run<'_> {
    fn emit_pages(&mut self, rendered: Vec<RgbaImage>) -> Result<()> {
        let t = Instant::now();
        let settings = self.settings;
        let encoded: Vec<PageImage> = rendered
            .par_iter()
            .map(|img| {
                process_page(img, &settings).map(|e| PageImage {
                    jpeg: e.jpeg,
                    width: e.width,
                    height: e.height,
                })
            })
            .collect::<Result<_>>()?;
        self.stats.filters_ms += t.elapsed().as_millis();
        self.pages.extend(encoded);
        self.stats.pages = self.pages.len();
        (self.progress)(Progress::Pages {
            done: self.pages.len(),
        });
        Ok(())
    }

    fn feed_strip(&mut self, img: &RgbaImage, last: bool) -> Result<()> {
        let t = Instant::now();
        let pages = self.paginator.feed(img, last)?;
        self.stats.pagination_ms += t.elapsed().as_millis();
        self.stats.strips += 1;
        self.stats.source_rows += img.height() as usize;
        self.emit_pages(pages.into_iter().map(|p| p.image).collect())
    }

    fn flush_strip(&mut self) -> Result<()> {
        let t = Instant::now();
        let pages = self.paginator.flush()?;
        self.stats.pagination_ms += t.elapsed().as_millis();
        self.emit_pages(pages.into_iter().map(|p| p.image).collect())
    }

    fn chapter(&mut self, dir: &Path, dev: &Device) -> Result<()> {
        let ChapterFiles {
            pages: files,
            skipped,
        } = chapter_files(dir)?;
        for file in &skipped {
            tracing::warn!(file = %file.display(), "export: skipping unsupported file");
        }
        self.stats.skipped += skipped.len();
        if files.is_empty() {
            self.stats.empty_chapters.push(dir.to_path_buf());
        }
        for (i, file) in files.iter().enumerate() {
            let img = decode(file)?;
            let (w, h) = (img.width(), img.height());
            if is_long_strip(w, h, dev.width, dev.height) || self.paginator.has_pending(w) {
                self.feed_strip(&img, i + 1 == files.len())?;
                continue;
            }
            self.flush_strip()?;
            self.emit_pages(vec![img])?;
        }
        self.flush_strip()
    }
}

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
    let mut paginator = StripPaginator::new(dev.width, dev.height, req.cut);
    if let Some(p) = penalizer {
        paginator = paginator.with_penalizer(p);
    }
    let mut run = Run {
        settings: ImageSettings::for_device(&dev),
        paginator,
        pages: Vec::new(),
        stats: ExportStats {
            pages: 0,
            strips: 0,
            source_rows: 0,
            pagination_ms: 0,
            filters_ms: 0,
            skipped: 0,
            empty_chapters: Vec::new(),
            output: PathBuf::new(),
        },
        progress,
    };
    let mut chapters = Vec::new();
    let total = req.chapter_dirs.len();
    for (index, dir) in req.chapter_dirs.iter().enumerate() {
        (run.progress)(Progress::Chapter {
            index,
            total,
            path: dir.clone(),
        });
        let first_page = run.pages.len();
        run.chapter(dir, &dev)
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
    run.stats.output = exporter.export(run.pages, &meta, &dev, &req.output)?;
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
