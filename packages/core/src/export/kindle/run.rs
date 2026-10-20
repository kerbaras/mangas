//! One pass of the page pipeline over chapter folders: decode, (trim), cut long strips, apply
//! the e-ink filters and encode. Shared by the export and the preview so both give the same pages.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use image::RgbaImage;
use rayon::prelude::*;

use super::input::{chapter_files, decode, ChapterFiles};
use super::process::{process_page, ImageSettings};
use super::request::{ExportStats, Progress};
use super::trim::{crop_sides, narrower, side_margins, trim_page};
use super::Device;
use crate::domain::{Error, Result};
use crate::export::epub::PageImage;
use crate::export::PageSettings;
use crate::smartcut::{is_long_strip, Options, RowPenalizer, StripPaginator};

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
    }
}

/// Side margins blank in every image of a width, so strip chunks keep stitching together.
/// Margin scans look at this many files at most, spread over the chapter, so a preview of a long
/// chapter doesn't decode every image first (and export and preview still agree).
const MARGIN_SAMPLE: usize = 24;

fn margin_sample(files: &[PathBuf]) -> Vec<&PathBuf> {
    let step = files.len().div_ceil(MARGIN_SAMPLE).max(1);
    files.iter().step_by(step).collect()
}

fn strip_margins(files: &[PathBuf]) -> Result<HashMap<u32, (u32, u32)>> {
    let found: Vec<(u32, Option<(u32, u32)>)> = margin_sample(files)
        .par_iter()
        .map(|f| decode(f).map(|img| (img.width(), side_margins(&img))))
        .collect::<Result<_>>()?;
    let mut by_width: HashMap<u32, Option<(u32, u32)>> = HashMap::new();
    for (w, m) in found {
        let entry = by_width.entry(w).or_insert(m);
        *entry = narrower(*entry, m);
    }
    Ok(by_width
        .into_iter()
        .filter_map(|(w, m)| m.map(|m| (w, m)))
        .collect())
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

    fn full(&self) -> bool {
        self.limit.is_some_and(|n| self.pages.len() >= n)
    }

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
        for (i, file) in files.iter().enumerate() {
            if (self.cancelled)() {
                return Err(Error::Other("cancelled".into()));
            }
            if self.full() {
                self.stopped_early = true;
                return Ok(());
            }
            let mut img = decode(file)?;
            if let Some(&m) = margins.get(&img.width()) {
                img = crop_sides(&img, m);
            }
            let (w, h) = (img.width(), img.height());
            if is_long_strip(w, h, dev.width, dev.height) || self.paginator.has_pending(w) {
                self.feed_strip(&img, i + 1 == files.len())?;
                continue;
            }
            self.flush_strip()?;
            let page = if self.trim { trim_page(&img) } else { img };
            self.emit_pages(vec![page])?;
        }
        if self.full() {
            self.stopped_early = self.paginator.has_pending_any();
            return Ok(());
        }
        self.flush_strip()
    }
}
