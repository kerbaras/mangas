//! Long blank gutters: a page that starts inside one skips its blank rows, so the blank space
//! never becomes pages of its own. Only gutters of the page background count: a flat band of
//! another colour (a black transition, a solid panel) is art.

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

/// Gutter rows per flat colour, summed over the strip so far.
pub type Votes = Vec<([u8; 3], usize)>;

fn same(p: [u8; 3], q: [u8; 3]) -> bool {
    (0..3).all(|c| (p[c] as i32 - q[c] as i32).abs() <= SAME_COLOUR)
}

/// Gutters split where the flat colour changes.
fn blank_runs(gutters: &[RowSpan], colours: &[[u8; 3]]) -> Vec<RowSpan> {
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

pub fn add_votes(mut votes: Votes, gutters: &[RowSpan], colours: &[[u8; 3]]) -> Votes {
    for g in blank_runs(gutters, colours) {
        let Some(&c) = colours.get(g.start) else {
            continue;
        };
        match votes.iter_mut().find(|(v, _)| same(*v, c)) {
            Some((_, n)) => *n += g.end - g.start,
            None => votes.push((c, g.end - g.start)),
        }
    }
    votes
}

/// The page background: the colour with the most gutter rows.
pub fn background(votes: &Votes) -> Option<[u8; 3]> {
    votes.iter().max_by_key(|(_, n)| *n).map(|(c, _)| *c)
}

fn is_bg(colours: &[[u8; 3]], y: usize, bg: Option<[u8; 3]>) -> bool {
    colours.get(y).zip(bg).is_some_and(|(c, b)| same(*c, b))
}

/// For every cut row `a` (`0..=height`), the first row of the page that starts at `a`. A cut
/// inside a long blank gutter starts the next page just above the art below it; one that runs
/// to the end of the strip is dropped entirely (`height`).
pub fn page_tops(
    gutters: &[RowSpan],
    colours: &[[u8; 3]],
    bg: Option<[u8; 3]>,
    height: usize,
    cap: usize,
) -> Vec<usize> {
    let mut tops: Vec<usize> = (0..=height).collect();
    let min_len = ((cap as f64 * LONG_GUTTER).ceil() as usize).max(1);
    let keep = (cap as f64 * KEEP).round() as usize;
    for g in blank_runs(gutters, colours)
        .iter()
        .filter(|g| g.end - g.start >= min_len && is_bg(colours, g.start, bg))
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
pub fn is_blank(
    f: &StripFeatures,
    colours: &[[u8; 3]],
    bg: Option<[u8; 3]>,
    sp: PageSpan,
    cap: usize,
) -> bool {
    let rows = sp.start..sp.end.min(f.height);
    if rows
        .clone()
        .any(|y| f.row_text[y] > 0.0 || f.row_bubble[y] > 0.0)
    {
        return false;
    }
    let shown = rows
        .filter(|&y| !(f.is_gutter[y] && is_bg(colours, y, bg)))
        .count();
    shown < (cap as f64 * MIN_CONTENT).ceil() as usize
}
