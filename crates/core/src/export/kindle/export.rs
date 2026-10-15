//! Chapter directories -> device pages -> fixed-layout EPUB. Long strips go through the smart
//! cutter (stitched across files); regular pages are scaled to fit.

use std::path::{Path, PathBuf};
use std::time::Instant;

use image::RgbaImage;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportStats {
    pub pages: usize,
    pub strips: usize,
    pub source_rows: usize,
    pub pagination_ms: u128,
    pub filters_ms: u128,
}

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// Image files of a chapter directory in name order.
pub fn chapter_images(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    files.sort();
    Ok(files)
}

pub fn decode(path: &Path) -> Result<RgbaImage> {
    Ok(image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .to_rgba8())
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
        chapters.push(ChapterEntry {
            title: chapter_title(dir, index),
            first_page: run.pages.len(),
        });
        run.chapter(dir, &dev)
            .map_err(|e| Error::Other(format!("{}: {e}", dir.display())))?;
    }
    (run.progress)(Progress::Writing {
        pages: run.pages.len(),
    });
    let book = Book {
        id: uuid_like(&req.title, run.pages.len()),
        title: req.title.clone(),
        author: "mangas".into(),
        language: "en".into(),
        right_to_left: false,
        device_width: dev.width,
        device_height: dev.height,
        chapters,
        pages: run.pages,
    };
    epub::write(&book, &req.output)?;
    Ok(run.stats)
}

fn chapter_title(dir: &Path, index: usize) -> String {
    dir.file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("Chapter {}", index + 1))
}

/// Deterministic identifier derived from the title and page count (no extra dependency).
fn uuid_like(title: &str, pages: usize) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(format!("{title}:{pages}"));
    let h: Vec<String> = hash.iter().take(16).map(|b| format!("{b:02x}")).collect();
    let s = h.concat();
    format!(
        "{}-{}-{}-{}-{}",
        &s[0..8],
        &s[8..12],
        &s[12..16],
        &s[16..20],
        &s[20..32]
    )
}
