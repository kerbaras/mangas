//! Turns a sequence of same-width strip images (one chapter, in reading order) into device
//! pages. Chapters arrive as arbitrary ~10 000 px chunks, so the unfinished tail of each image is
//! carried over to the next one; a panel straddling two files is never cut at the file boundary.

use image::RgbaImage;
use rayon::prelude::*;

use super::costs::cut_costs;
use super::features::{self, RowSpan, StripFeatures};
use super::gutters::{add_votes, background, is_blank, page_tops, row_colours, Votes};
use super::paginate::{paginate_rows, PageSpan};
use super::raster::{crop_rows, resized_for_analysis, stack, RgbStrip};
use super::render::render_page;
use super::Options;
use crate::domain::Result;

/// Strips wider than this are downscaled for analysis only.
pub const ANALYSIS_WIDTH: u32 = 720;
/// An image is a strip if it needs more than this many pages.
const LONG_STRIP_MIN_PAGES: f64 = 1.25;

/// Extra per-row cut cost for an analysis-sized strip (the ML detectors plug in here).
pub type RowPenalizer = std::sync::Arc<dyn Fn(&RgbaImage) -> Result<Vec<f32>> + Send + Sync>;

/// A rendered device page and the strip rows it shows (global rows across all fed images).
#[derive(Debug, Clone)]
pub struct Page {
    pub image: RgbaImage,
    pub rows: RowSpan,
}

pub struct StripPaginator {
    device_w: u32,
    device_h: u32,
    opts: Options,
    penalizer: Option<RowPenalizer>,
    carry: Option<RgbaImage>,
    /// Global row of the top of the pending unit.
    offset: usize,
    votes: Votes,
}

pub fn is_long_strip(w: u32, h: u32, device_w: u32, device_h: u32) -> bool {
    w > 0
        && h > 0
        && (h as f64 / w as f64) > (device_h as f64 / device_w as f64) * LONG_STRIP_MIN_PAGES
}

/// Rows of a `strip_w`-wide strip that fit on a page at native zoom.
pub fn capacity(strip_w: usize, device_w: u32, device_h: u32) -> usize {
    ((strip_w as f64 * device_h as f64 / device_w as f64).floor() as usize).max(1)
}

impl StripPaginator {
    pub fn new(device_w: u32, device_h: u32, opts: Options) -> Self {
        StripPaginator {
            device_w,
            device_h,
            opts,
            penalizer: None,
            carry: None,
            offset: 0,
            votes: Votes::new(),
        }
    }

    pub fn with_penalizer(mut self, p: RowPenalizer) -> Self {
        self.penalizer = Some(p);
        self
    }

    /// Adds the next strip image and returns the pages that are final so far. Unless `last`,
    /// the tail of the strip is held back until the next `feed` (or `flush`).
    pub fn feed(&mut self, img: &RgbaImage, last: bool) -> Result<Vec<Page>> {
        let mut out = Vec::new();
        if self
            .carry
            .as_ref()
            .is_some_and(|c| c.width() != img.width())
        {
            out.extend(self.flush()?);
        }
        let unit = stack(self.carry.take().as_ref(), img);
        out.extend(self.paginate_unit(&unit, last)?);
        Ok(out)
    }

    /// Whether rows are held back waiting for the next chunk of this width.
    pub fn has_pending(&self, width: u32) -> bool {
        self.carry.as_ref().is_some_and(|c| c.width() == width)
    }

    /// Whether any rows are held back for a next chunk.
    pub fn has_pending_any(&self) -> bool {
        self.carry.is_some()
    }

    pub fn flush(&mut self) -> Result<Vec<Page>> {
        match self.carry.take() {
            Some(unit) => self.paginate_unit(&unit, true),
            None => Ok(Vec::new()),
        }
    }

    fn paginate_unit(&mut self, unit: &RgbaImage, last: bool) -> Result<Vec<Page>> {
        let (w, h) = (unit.width() as usize, unit.height() as usize);
        if w == 0 || h == 0 {
            return Ok(Vec::new());
        }
        let analysis = resized_for_analysis(unit, ANALYSIS_WIDTH)?;
        let strip = RgbStrip::from_rgba(&analysis);
        let feats = features::compute(&strip);
        let extra = self.penalizer.as_ref().map(|p| p(&analysis)).transpose()?;
        let to_src = h as f64 / strip.height as f64;
        let cap = capacity(strip.width, self.device_w, self.device_h);
        let overlap = ((cap as f64 * self.opts.overlap).round() as usize).min(cap / 2);
        let cost = cut_costs(&feats, extra.as_deref());
        let colours = row_colours(&strip);
        self.votes = add_votes(std::mem::take(&mut self.votes), &feats.gutters, &colours);
        let bg = background(&self.votes);
        let tops = page_tops(&feats.gutters, &colours, bg, feats.height, cap);
        let mut spans = paginate_rows(
            &cost,
            &tops,
            cap,
            self.opts.max_shrink,
            self.opts.shrink_penalty,
            overlap,
        );
        // A strip ending in a skipped gutter has no tail to hold back.
        let ends_blank = spans.last().is_none_or(|sp| sp.end < feats.height);
        if !last && !ends_blank {
            if spans.len() <= 1 {
                self.carry = Some(unit.clone());
                return Ok(Vec::new());
            }
            let tail = spans.pop().unwrap_or(PageSpan { start: 0, end: 0 });
            spans.retain(|sp| !is_blank(&feats, &colours, bg, *sp, cap));
            let start = ((tail.start as f64 * to_src).round() as usize).min(h - 1);
            self.carry = Some(crop_rows(unit, start as u32, h as u32));
            let pages = self.render(unit, &feats, &spans, to_src)?;
            self.offset += start;
            return Ok(pages);
        }
        spans.retain(|sp| !is_blank(&feats, &colours, bg, *sp, cap));
        let pages = self.render(unit, &feats, &spans, to_src)?;
        self.offset += h;
        Ok(pages)
    }

    fn render(
        &self,
        unit: &RgbaImage,
        f: &StripFeatures,
        spans: &[PageSpan],
        to_src: f64,
    ) -> Result<Vec<Page>> {
        let h = unit.height() as usize;
        let jobs: Vec<(PageSpan, [u8; 3])> = spans
            .iter()
            .filter_map(|sp| {
                let a = ((sp.start as f64 * to_src).round() as usize).min(h);
                let b = if sp.end == f.height {
                    h
                } else {
                    ((sp.end as f64 * to_src).round() as usize).min(h)
                };
                (b > a).then(|| {
                    (
                        PageSpan { start: a, end: b },
                        f.bg[sp.start.min(f.height - 1)],
                    )
                })
            })
            .collect();
        let (dev_w, dev_h, offset) = (self.device_w, self.device_h, self.offset);
        jobs.into_par_iter()
            .map(|(sp, bg)| {
                Ok(Page {
                    image: render_page(unit, sp, bg, dev_w, dev_h)?,
                    rows: RowSpan {
                        start: offset + sp.start,
                        end: offset + sp.end,
                    },
                })
            })
            .collect()
    }
}
