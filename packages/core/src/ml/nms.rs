//! Detection boxes and class-agnostic non-maximum suppression. Pure functions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Bubble,
    Text,
    Face,
}

/// A detection in strip pixel coordinates (x0, y0 inclusive; x1, y1 exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Detection {
    pub class: Class,
    pub score: f32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Detection {
    pub fn area(&self) -> f32 {
        (self.x1 - self.x0).max(0.0) * (self.y1 - self.y0).max(0.0)
    }

    pub fn iou(&self, o: &Detection) -> f32 {
        let ix = (self.x1.min(o.x1) - self.x0.max(o.x0)).max(0.0);
        let iy = (self.y1.min(o.y1) - self.y0.max(o.y0)).max(0.0);
        let inter = ix * iy;
        let union = self.area() + o.area() - inter;
        if union <= 0.0 {
            0.0
        } else {
            inter / union
        }
    }

    pub fn shifted(self, dy: f32) -> Detection {
        Detection {
            y0: self.y0 + dy,
            y1: self.y1 + dy,
            ..self
        }
    }
}

/// Greedy NMS per class: keep the highest score, drop boxes overlapping it by more than `iou`.
pub fn nms(mut dets: Vec<Detection>, iou: f32) -> Vec<Detection> {
    dets.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut kept: Vec<Detection> = Vec::new();
    for d in dets {
        if kept.iter().all(|k| k.class != d.class || k.iou(&d) <= iou) {
            kept.push(d);
        }
    }
    kept
}
