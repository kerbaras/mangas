//! Chapter directories -> device pages -> fixed-layout EPUB. Long strips go through the smart
//! cutter (stitched across files); regular pages are scaled to fit.

use std::path::{Path, PathBuf};
use std::time::Instant;

use image::RgbaImage;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::input::{chapter_images, chapter_title, decode, uuid_like};
use super::process::{process_page, ImageSettings};
use super::Device;
use crate::domain::{Error, Result};
use crate::export::epub::{self, Book, ChapterEntry, PageImage};
use crate::smartcut::{is_long_strip, Options, RowPenalizer, StripPaginator};

#[derive(Debug, Clone)]
pub struct ExportRequest {
    pub chapter_dirs: Vec<PathBuf>,
    pub device: Device,
    pub cut: Options,
    pub title: String,
    pub output: PathBuf,
}

/// Progress events, in order: one `Chapter` per directory, `Pages` as pages are finished, `Writing`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Progress {
    Chapter {
        index: usize,
        total: usize,
        path: PathBuf,
    },
    Pages {
        done: usize,
    },
    Writing {
        pages: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportStats {
    pub pages: usize,
    pub strips: usize,
    pub source_rows: usize,
    pub pagination_ms: u128,
    pub filters_ms: u128,
    /// File actually written (differs from the request when the name was already taken).
    pub output: PathBuf,
}

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
        let files = chapter_images(dir)?;
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
            chapters.push(ChapterEntry {
                title: chapter_title(dir, index),
                first_page,
            });
        }
    }
    (run.progress)(Progress::Writing {
        pages: run.pages.len(),
    });
    let book = Book {
        id: uuid_like(&req.title, &req.output),
        title: req.title.clone(),
        author: "mangas".into(),
        language: "en".into(),
        right_to_left: false,
        device_width: dev.width,
        device_height: dev.height,
        chapters,
        pages: run.pages,
    };
    run.stats.output = epub::write(&book, &req.output)?;
    Ok(run.stats)
}
