//! Long blank gutters: a page that starts inside one skips its blank rows, so the blank space
//! never becomes pages of its own.

use super::features::{RowSpan, StripFeatures};
use super::paginate::PageSpan;
use super::raster::RgbStrip;

/// A gutter at least this fraction of a page tall is skipped.
const LONG_GUTTER: f64 = 0.25;
/// Blank rows kept above the art when a page starts inside a long gutter (fraction of a page).
const KEEP: f64 = 0.03;
/// A page showing anything on fewer rows than this fraction of a page (a stray border line or
/// speck at the edge of a long gutter), and no text, is dropped.
const MIN_CONTENT: f64 = 0.02;
/// Flat rows whose colour differs by more than this from the run's first row start a new run.
const SAME_COLOUR: i32 = 8;

/// Colour of each row, sampled in the middle (gutter rows are flat, so any pixel will do).
pub fn row_colours(s: &RgbStrip) -> Vec<[u8; 3]> {
    (0..s.height).map(|y| s.px(s.width / 2, y)).collect()
}

/// Gutters split where the flat colour changes: stacked bands of solid colour are art.
fn same(p: [u8; 3], q: [u8; 3]) -> bool {
    (0..3).all(|c| (p[c] as i32 - q[c] as i32).abs() <= SAME_COLOUR)
}

fn blank_runs(gutters: &[RowSpan], colours: &[[u8; 3]]) -> Vec<RowSpan> {
    let same =
        |p: [u8; 3], q: [u8; 3]| (0..3).all(|c| (p[c] as i32 - q[c] as i32).abs() <= SAME_COLOUR);
    let mut out = Vec::new();
    for g in gutters {
        let mut start = g.start;
        for y in g.start..g.end.min(colours.len()) {
            if !same(colours[start], colours[y]) {
                out.push(RowSpan { start, end: y });
                start = y;
            }
        }
        out.push(RowSpan { start, end: g.end });
    }
    out
}

/// For every cut row `a` (`0..=height`), the first row of the page that starts at `a`. A cut
/// inside a long blank gutter starts the next page just above the art below it; one that runs
/// to the end of the strip is dropped entirely (`height`).
pub fn page_tops(
    gutters: &[RowSpan],
    colours: &[[u8; 3]],
    height: usize,
    cap: usize,
) -> Vec<usize> {
    let mut tops: Vec<usize> = (0..=height).collect();
    let min_len = ((cap as f64 * LONG_GUTTER).ceil() as usize).max(1);
    let keep = (cap as f64 * KEEP).round() as usize;
    for g in blank_runs(gutters, colours)
        .iter()
        .filter(|g| g.end - g.start >= min_len)
    {
        let end = g.end.min(height);
        let to = if end == height { height } else { end - keep };
        for top in &mut tops[g.start..end] {
            *top = (*top).max(to);
        }
    }
    tops
}

/// Whether a page shows (almost) nothing but its background: flat rows of the background colour
/// do not count, and a detected text line or bubble always does.
pub fn is_blank(f: &StripFeatures, colours: &[[u8; 3]], sp: PageSpan, cap: usize) -> bool {
    let rows = sp.start..sp.end.min(f.height);
    if rows
        .clone()
        .any(|y| f.row_text[y] > 0.0 || f.row_bubble[y] > 0.0)
    {
        return false;
    }
    let shown = rows
        .filter(|&y| !(f.is_gutter[y] && colours.get(y).is_some_and(|c| same(*c, f.bg[y]))))
        .count();
    shown < (cap as f64 * MIN_CONTENT).ceil() as usize
}
