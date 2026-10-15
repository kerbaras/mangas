//! Detections -> extra per-row cut costs for smartcut (same weights as the Go version).

use super::nms::{Class, Detection};
use crate::smartcut::costs::{CUT_COST_BUBBLE, CUT_COST_FACE, CUT_COST_TEXT};

pub fn weight(c: Class) -> f32 {
    match c {
        Class::Bubble => CUT_COST_BUBBLE as f32,
        Class::Text => CUT_COST_TEXT as f32,
        Class::Face => CUT_COST_FACE as f32,
    }
}

/// Per-row penalty: the maximum weight of any detection covering that row.
pub fn row_penalties(dets: &[Detection], height: usize) -> Vec<f32> {
    let mut rows = vec![0f32; height];
    for d in dets {
        let y0 = d.y0.floor().max(0.0) as usize;
        let y1 = (d.y1.ceil().max(0.0) as usize).min(height);
        let w = weight(d.class);
        for r in rows.iter_mut().take(y1).skip(y0) {
            *r = r.max(w);
        }
    }
    rows
}
