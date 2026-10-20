//! Per-row features of a strip: gutters, background colour, panel / text / bubble coverage and
//! texture. Everything the cut-cost function needs, computed once per strip.

use serde::{Deserialize, Serialize};

use super::bubbles;
use super::components::{self, PixelBox};
use super::morphology;
use super::raster::RgbStrip;
use super::rows::{row_texture, row_uniformity, rows_covered, width_covered};
use super::text;

const GUTTER_STD_THRESHOLD: f64 = 3.0;
const GUTTER_MIN_ROWS: usize = 6;
const FG_TOLERANCE: i32 = 18;
const PANEL_CLOSE_KERNEL: usize = 15;
const PANEL_MIN_AREA: usize = 1500;

/// Half-open `[start, end)` range of rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StripFeatures {
    pub height: usize,
    pub width: usize,
    pub is_gutter: Vec<bool>,
    /// Background colour per row (colour of the nearest gutter).
    pub bg: Vec<[u8; 3]>,
    /// Mean |horizontal luma gradient| per row, normalised to mean 1.
    pub texture: Vec<f32>,
    pub row_panel: Vec<f32>,
    pub row_text: Vec<f32>,
    pub row_bubble: Vec<f32>,
    pub gutters: Vec<RowSpan>,
    pub panels: Vec<PixelBox>,
    pub text: Vec<PixelBox>,
    pub bubbles: Vec<PixelBox>,
}

pub fn compute(s: &RgbStrip) -> StripFeatures {
    let gutters = find_gutters(&row_uniformity(s));
    let mut is_gutter = vec![false; s.height];
    for g in &gutters {
        is_gutter[g.start..g.end].iter_mut().for_each(|v| *v = true);
    }
    let bg = background_per_row(s, &gutters);
    let panels = panel_boxes(&foreground_mask(s, &bg), s.width, s.height);
    let text = text::text_boxes(s);
    let bubbles = text
        .iter()
        .map(|t| bubbles::bubble_around(s, *t))
        .collect::<Vec<_>>();
    StripFeatures {
        height: s.height,
        width: s.width,
        is_gutter,
        bg,
        texture: row_texture(s),
        row_panel: rows_covered(&panels, s.height),
        row_text: rows_covered(&text, s.height),
        row_bubble: bubble_rows(&bubbles, s.width, s.height),
        gutters,
        panels,
        text,
        bubbles,
    }
}

/// Share of each row's width inside a balloon, never under `MIN_BUBBLE` on a balloon row, so
/// even a narrow balloon costs more than a page shrunk into its slack.
fn bubble_rows(bubbles: &[PixelBox], w: usize, h: usize) -> Vec<f32> {
    const MIN_BUBBLE: f32 = 0.5;
    let floor = |c: f32| if c > 0.0 { c.max(MIN_BUBBLE) } else { 0.0 };
    width_covered(bubbles, w, h)
        .into_iter()
        .map(floor)
        .collect()
}

pub fn find_gutters(uni: &[f64]) -> Vec<RowSpan> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for y in 0..=uni.len() {
        let flat = y < uni.len() && uni[y] < GUTTER_STD_THRESHOLD;
        match (flat, start) {
            (true, None) => start = Some(y),
            (false, Some(a)) => {
                if y - a >= GUTTER_MIN_ROWS {
                    out.push(RowSpan { start: a, end: y });
                }
                start = None;
            }
            _ => {}
        }
    }
    out
}

/// Per-channel median colour of rows `[a, b)`.
pub fn median_colour(s: &RgbStrip, a: usize, b: usize) -> [u8; 3] {
    let mut hist = [[0usize; 256]; 3];
    for y in a..b {
        for p in s.row(y).as_chunks::<3>().0.iter() {
            for c in 0..3 {
                hist[c][p[c] as usize] += 1;
            }
        }
    }
    let total = (b.saturating_sub(a)) * s.width;
    let mut out = [0u8; 3];
    for c in 0..3 {
        let mut acc = 0usize;
        for (v, n) in hist[c].iter().enumerate() {
            acc += n;
            if acc * 2 >= total {
                out[c] = v as u8;
                break;
            }
        }
    }
    out
}

/// Every row gets the colour of the nearest gutter; without gutters, the left margin's median.
pub fn background_per_row(s: &RgbStrip, gutters: &[RowSpan]) -> Vec<[u8; 3]> {
    if gutters.is_empty() {
        let edge = s.left_columns(10);
        return vec![median_colour(&edge, 0, s.height); s.height];
    }
    let centers: Vec<i64> = gutters
        .iter()
        .map(|g| ((g.start + g.end) / 2) as i64)
        .collect();
    let cols: Vec<[u8; 3]> = gutters
        .iter()
        .map(|g| median_colour(s, g.start, g.end))
        .collect();
    let mut j = 0;
    (0..s.height as i64)
        .map(|y| {
            while j + 1 < centers.len() && (centers[j + 1] - y).abs() <= (centers[j] - y).abs() {
                j += 1;
            }
            cols[j]
        })
        .collect()
}

pub fn foreground_mask(s: &RgbStrip, bg: &[[u8; 3]]) -> Vec<u8> {
    let mut m = vec![0u8; s.width * s.height];
    for y in 0..s.height {
        let b = bg[y];
        for (x, p) in s.row(y).as_chunks::<3>().0.iter().enumerate() {
            let d = (0..3)
                .map(|c| (p[c] as i32 - b[c] as i32).abs())
                .max()
                .unwrap_or(0);
            if d > FG_TOLERANCE {
                m[y * s.width + x] = 1;
            }
        }
    }
    m
}

pub fn panel_boxes(fg: &[u8], w: usize, h: usize) -> Vec<PixelBox> {
    let closed = morphology::close(fg, w, h, PANEL_CLOSE_KERNEL / 2);
    let (_, comps) = components::label(&closed, w, h);
    comps
        .into_iter()
        .filter(|c| c.area >= PANEL_MIN_AREA && c.bbox.width() >= 20 && c.bbox.height() >= 20)
        .map(|c| c.bbox)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_narrow_balloon_still_costs_half_a_row() {
        let narrow = [PixelBox {
            x0: 0,
            y0: 1,
            x1: 10,
            y1: 2,
        }];
        assert_eq!(bubble_rows(&narrow, 100, 3), vec![0.0, 0.5, 0.0]);
    }
}
