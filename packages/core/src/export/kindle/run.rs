//! One pass of the page pipeline over chapter folders: decode, (trim), cut long strips, apply
//! the e-ink filters and encode. Shared by the export and the preview so both give the same pages.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use image::RgbaImage;
use rayon::prelude::*;

use super::input::{chapter_files, decode, ChapterFiles};
use super::margins::strip_margins;
use super::process::{process_page, ImageSettings};
use super::request::{ExportStats, Progress};
use super::trim::{crop_sides, trim_page};
use super::Device;
use crate::domain::{Error, Result};
use crate::export::epub::PageImage;
use crate::export::PageSettings;
use crate::smartcut::{is_long_strip, Options, RowPenalizer, StripPaginator};

/// Full-size scans decoded at once: most of the multi-core speed-up at a bounded peak memory.
const WINDOW: usize = 4;

pub(super) struct Run<'a> {
    pub settings: ImageSettings,
    pub paginator: StripPaginator,
    pub pages: Vec<PageImage>,
    pub stats: ExportStats,
    pub progress: &'a mut dyn FnMut(Progress),
    device: Device,
    trim: bool,
    /// Stop once this many pages are final (preview); the rest of the chapter is not read.
    limit: Option<usize>,
    cancelled: &'a dyn Fn() -> bool,
    pub stopped_early: bool,
}

pub(super) struct RunSpec<'a> {
    pub device: Device,
    pub cut: Options,
    pub page: PageSettings,
    pub penalizer: Option<RowPenalizer>,
    pub limit: Option<usize>,
    pub cancelled: &'a dyn Fn() -> bool,
}

fn empty_stats() -> ExportStats {
    ExportStats {
        pages: 0,
        strips: 0,
        source_rows: 0,
        pagination_ms: 0,
        filters_ms: 0,
        skipped: 0,
        empty_chapters: Vec::new(),
        output: PathBuf::new(),
        parts: Vec::new(),
        detectors_error: None,
        oversized: Vec::new(),
    }
}

impl<'a> Run<'a> {
    pub fn new(spec: RunSpec<'a>, progress: &'a mut dyn FnMut(Progress)) -> Run<'a> {
        let dev = spec.device;
        let mut paginator = StripPaginator::new(dev.width, spec.page.page_height(&dev), spec.cut);
        if let Some(p) = spec.penalizer {
            paginator = paginator.with_penalizer(p);
        }
        Run {
            settings: spec.page.image_settings(&dev),
            paginator,
            pages: Vec::new(),
            stats: empty_stats(),
            progress,
            device: dev,
            trim: spec.page.trim_margins,
            limit: spec.limit,
            cancelled: spec.cancelled,
            stopped_early: false,
        }
    }

    /// Full counting `queued` pages not emitted yet.
    fn full_with(&self, queued: usize) -> bool {
        self.limit.is_some_and(|n| self.pages.len() + queued >= n)
    }

    /// Trims (when asked) and emits regular pages, in order.
    fn emit_regular(&mut self, batch: Vec<RgbaImage>) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }
        self.emit_pages(batch, self.trim)
    }

    /// Each page is dropped as soon as it is encoded, so a window holds few full-size copies.
    fn emit_pages(&mut self, rendered: Vec<RgbaImage>, trim: bool) -> Result<()> {
        let t = Instant::now();
        let settings = self.settings;
        let encoded: Vec<PageImage> = rendered
            .into_par_iter()
            .map(|img| {
                let img = if trim { trim_page(&img) } else { img };
                process_page(&img, &settings).map(|e| PageImage {
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
        self.emit_pages(pages.into_iter().map(|p| p.image).collect(), false)
    }

    fn flush_strip(&mut self) -> Result<()> {
        let t = Instant::now();
        let pages = self.paginator.flush()?;
        self.stats.pagination_ms += t.elapsed().as_millis();
        self.emit_pages(pages.into_iter().map(|p| p.image).collect(), false)
    }

    pub fn chapter(&mut self, dir: &Path) -> Result<()> {
        self.files(dir, chapter_files(dir)?)
    }

    /// Runs `files` as the content of chapter `dir`.
    pub fn files(&mut self, dir: &Path, files: ChapterFiles) -> Result<()> {
        self.paginator.new_chapter();
        let ChapterFiles {
            pages: files,
            skipped,
        } = files;
        for file in &skipped {
            tracing::warn!(file = %file.display(), "export: skipping unsupported file");
        }
        self.stats.skipped += skipped.len();
        if files.is_empty() {
            self.stats.empty_chapters.push(dir.to_path_buf());
        }
        let margins = if self.trim {
            strip_margins(&files)?
        } else {
            HashMap::new()
        };
        let dev = self.device;
        // Regular pages are independent: decode and process a window of them in parallel.
        let window = rayon::current_num_threads().clamp(1, WINDOW);
        for (c, chunk) in files.chunks(window).enumerate() {
            let images: Vec<RgbaImage> = chunk
                .par_iter()
                .map(|file| {
                    let img = decode(file)?;
                    Ok(match margins.get(&img.width()) {
                        Some(&m) => crop_sides(&img, m),
                        None => img,
                    })
                })
                .collect::<Result<_>>()?;
            let mut regular = Vec::new();
            for (k, img) in images.into_iter().enumerate() {
                if (self.cancelled)() {
                    return Err(Error::Other("cancelled".into()));
                }
                if self.full_with(regular.len()) {
                    self.emit_regular(regular)?;
                    self.stopped_early = true;
                    return Ok(());
                }
                let (w, h) = (img.width(), img.height());
                if is_long_strip(w, h, dev.width, dev.height) || self.paginator.has_pending(w) {
                    self.emit_regular(std::mem::take(&mut regular))?;
                    self.feed_strip(&img, c * window + k + 1 == files.len())?;
                    continue;
                }
                if regular.is_empty() {
                    self.flush_strip()?;
                }
                regular.push(img);
            }
            self.emit_regular(regular)?;
        }
        if self.full_with(0) {
            self.stopped_early = self.paginator.has_pending_any();
            return Ok(());
        }
        self.flush_strip()
    }
}
