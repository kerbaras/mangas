//! Cost of placing a page boundary just above each row.

use super::features::StripFeatures;

pub const CUT_COST_BASE: f64 = 0.5;
pub const CUT_COST_PANEL: f64 = 1.0;
pub const CUT_COST_BUBBLE: f64 = 30.0;
pub const CUT_COST_TEXT: f64 = 60.0;
pub const CUT_COST_FACE: f64 = 60.0;
pub const CUT_PAGE_COST: f64 = 1.0;
/// Any cut that is not on a gutter row counts as "through art".
pub const CUT_BAD_THRESHOLD: f64 = 0.01;

/// One entry per row plus a free entry at index `height` (end of strip). `extra` adds a per-row
/// penalty (for example from the ML detectors); it is ignored on gutter rows like everything else.
pub fn cut_costs(f: &StripFeatures, extra: Option<&[f32]>) -> Vec<f64> {
    let mut c = vec![0f64; f.height + 1];
    for (y, slot) in c.iter_mut().enumerate().take(f.height) {
        if f.is_gutter[y] {
            continue;
        }
        let bonus = extra.and_then(|e| e.get(y)).copied().unwrap_or(0.0) as f64;
        *slot = CUT_COST_BASE
            + f.texture[y] as f64
            + CUT_COST_PANEL * f.row_panel[y] as f64
            + CUT_COST_BUBBLE * f.row_bubble[y] as f64
            + CUT_COST_TEXT * f.row_text[y] as f64
            + bonus;
    }
    c
}
