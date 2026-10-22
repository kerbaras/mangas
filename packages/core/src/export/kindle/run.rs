//! One pass of the page pipeline over chapter folders: decode, (trim), cut long strips, apply
//! the e-ink filters and encode. Shared by the export and the preview so both give the same pages.

use std::path::{Path, PathBuf};
use std::time::Instant;

use image::RgbaImage;
use rayon::prelude::*;

use super::input::{chapter_files, ChapterFiles};
use super::margins::{load, strip_margins};
use super::plan::{page_size, regular_run, Margins};
use super::pool::{self, WORKERS as WINDOW};
use super::process::{encode_owned, ImageSettings};
use super::request::{ExportStats, Progress};
use super::trim::trim_owned;
use super::Device;
use crate::domain::{Error, Result};
use crate::export::epub::PageImage;
use crate::export::{PageSettings, PageSpool};
use crate::smartcut::{Options, RowPenalizer, StripPaginator};

pub(super) struct Run<'a> {
    pub settings: ImageSettings,
    pub paginator: StripPaginator,
    /// Finished pages, on disk until the format writer reads them back.
    pub pages: PageSpool,
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
    pub spool_dir: Option<&'a Path>,
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
        jpeg_quality: 0,
        chapters: Vec::new(),
    }
}

impl<'a> Run<'a> {
    pub fn new(spec: RunSpec<'a>, progress: &'a mut dyn FnMut(Progress)) -> Result<Run<'a>> {
        let dev = spec.device;
        let mut paginator = StripPaginator::new(dev.width, spec.page.page_height(&dev), spec.cut);
        if let Some(p) = spec.penalizer {
            paginator = paginator.with_penalizer(p);
        }
        Ok(Run {
            settings: spec.page.image_settings(&dev),
            paginator,
            pages: PageSpool::new_in(spec.spool_dir)?,
            stats: empty_stats(),
            progress,
            device: dev,
            trim: spec.page.trim_margins,
            limit: spec.limit,
            cancelled: spec.cancelled,
            stopped_early: false,
        })
    }

    /// Full counting `queued` pages not emitted yet.
    fn full_with(&self, queued: usize) -> bool {
        self.limit.is_some_and(|n| self.pages.len() + queued >= n)
    }

    /// Decodes, crops, trims and encodes a run (at most `WORKERS` pages) side by side, in order.
    fn emit_files(&mut self, files: &[PathBuf], margins: &Margins) -> Result<()> {
        let t = Instant::now();
        let (settings, trim) = (self.settings, self.trim);
        let encoded = pool::install(|| {
            files
                .par_iter()
                .map(|file| {
                    let img = load(file, margins)?;
                    encode_owned(if trim { trim_owned(img) } else { img }, &settings)
                })
                .collect::<Result<_>>()
        })?;
        self.push(encoded, t)
    }

    fn emit_pages(&mut self, rendered: Vec<RgbaImage>) -> Result<()> {
        let t = Instant::now();
        let settings = self.settings;
        let encoded = pool::install(|| {
            rendered
                .into_par_iter()
                .map(|img| encode_owned(img, &settings))
                .collect::<Result<_>>()
        })?;
        self.push(encoded, t)
    }

    fn check_cancelled(&self) -> Result<()> {
        match (self.cancelled)() {
            true => Err(Error::Other("cancelled".into())),
            false => Ok(()),
        }
    }

    fn push(&mut self, encoded: Vec<PageImage>, t: Instant) -> Result<()> {
        self.stats.filters_ms += t.elapsed().as_millis();
        for page in &encoded {
            self.pages.push(page)?;
        }
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
            Margins::new()
        };
        let device = (self.device.width, self.device.height);
        let mut i = 0;
        while i < files.len() {
            self.check_cancelled()?;
            if self.full_with(0) {
                self.stopped_early = true;
                return Ok(());
            }
            // Past a strip's held-back rows, a page of the same width continues that strip.
            let room = self
                .limit
                .map_or(WINDOW, |n| n.saturating_sub(self.pages.len()));
            let run = match self
                .paginator
                .has_pending(page_size(&files[i], &margins)?.0)
            {
                true => 0,
                false => regular_run(&files[i..], &margins, device, WINDOW.min(room))?,
            };
            if run == 0 {
                let img = load(&files[i], &margins)?;
                self.feed_strip(&img, i + 1 == files.len())?;
                i += 1;
                continue;
            }
            self.flush_strip()?;
            self.emit_files(&files[i..i + run], &margins)?;
            i += run;
        }
        self.check_cancelled()?;
        if self.full_with(0) {
            self.stopped_early = self.paginator.has_pending_any();
            return Ok(());
        }
        self.flush_strip()
    }
}
