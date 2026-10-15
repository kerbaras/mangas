//! Dynamic-programming choice of page boundaries over the per-row cut costs.

use serde::{Deserialize, Serialize};

use super::costs::{CUT_BAD_THRESHOLD, CUT_PAGE_COST};

/// A page as a half-open row range of the strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSpan {
    pub start: usize,
    pub end: usize,
}

/// Where a page starts when the previous cut was at row `a`: a cut through artwork repeats
/// `overlap` rows.
fn page_start(cost: &[f64], a: usize, overlap: usize) -> usize {
    if a > 0 && cost[a] > CUT_BAD_THRESHOLD {
        a.saturating_sub(overlap)
    } else {
        a
    }
}

/// Chooses page boundaries for a strip with `cost.len() - 1` rows where `cap` rows fit on a page
/// at native zoom. A page may hold up to `cap * max_shrink` rows at a cost of
/// `shrink_penalty * excess / cap`.
pub fn paginate_rows(
    cost: &[f64],
    cap: usize,
    max_shrink: f64,
    shrink_penalty: f64,
    overlap: usize,
) -> Vec<PageSpan> {
    let h = cost.len().saturating_sub(1);
    if h == 0 || cap == 0 {
        return Vec::new();
    }
    let max_shrink = if max_shrink >= 1.0 { max_shrink } else { 1.0 }; // also guards NaN
    let max_h = (cap as f64 * max_shrink) as usize;
    let mut best = vec![f64::INFINITY; h + 1];
    let mut prev = vec![usize::MAX; h + 1];
    best[0] = 0.0;
    for a in 0..h {
        if best[a].is_infinite() {
            continue;
        }
        let start = page_start(cost, a, overlap);
        let hi = h.min(start + max_h);
        for b in (a + 1)..=hi {
            let rows = b - start;
            let pen = if rows > cap {
                shrink_penalty * (rows - cap) as f64 / cap as f64
            } else {
                0.0
            };
            let tot = best[a] + CUT_PAGE_COST + cost[b] + pen;
            if tot < best[b] {
                best[b] = tot;
                prev[b] = a;
            }
        }
    }
    let mut pages = Vec::new();
    let mut b = h;
    while b > 0 && prev[b] != usize::MAX {
        let a = prev[b];
        pages.push(PageSpan {
            start: page_start(cost, a, overlap),
            end: b,
        });
        b = a;
    }
    pages.reverse();
    pages
}
