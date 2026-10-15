//! RT-DETR bubble/text detector and anime face detector over tiled strips.

use std::path::Path;
use std::sync::Mutex;

use image::RgbaImage;
use ort::session::Session;
use ort::value::Tensor;

use super::nms::{nms, Class, Detection};
use super::penalty::row_penalties;
use super::preprocess::{chw, prepare, Prepared, INPUT_SIZE};
use super::runtime::{load_session, ort_err};
use super::tiles::windows;
use crate::domain::{Error, Result};
use crate::smartcut::raster::crop_rows;

pub const TILE_SIZE: usize = 1024;
pub const TILE_OVERLAP: usize = 128;
pub const DETECTOR_SCORE_THRESHOLD: f32 = 0.4;
pub const NMS_IOU: f32 = 0.5;

pub struct Detectors {
    bubble_text: Mutex<Session>,
    face: Mutex<Session>,
}

impl Detectors {
    pub fn load(bubble_text: &Path, face: &Path) -> Result<Self> {
        Ok(Detectors {
            bubble_text: Mutex::new(load_session(bubble_text)?),
            face: Mutex::new(load_session(face)?),
        })
    }

    /// All detections on a strip, in strip coordinates, after NMS.
    pub fn detect(&self, strip: &RgbaImage) -> Result<Vec<Detection>> {
        let mut all = Vec::new();
        for (y0, y1) in windows(strip.height() as usize, TILE_SIZE, TILE_OVERLAP) {
            let tile = crop_rows(strip, y0 as u32, y1 as u32);
            let prepared = prepare(&tile)?;
            let mut dets = self.run_bubble_text(&prepared)?;
            dets.extend(self.run_face(&prepared)?);
            all.extend(dets.into_iter().map(|d| d.shifted(y0 as f32)));
        }
        Ok(nms(all, NMS_IOU))
    }

    pub fn row_penalties(&self, strip: &RgbaImage) -> Result<Vec<f32>> {
        Ok(row_penalties(&self.detect(strip)?, strip.height() as usize))
    }

    fn run_bubble_text(&self, p: &Prepared) -> Result<Vec<Detection>> {
        let n = INPUT_SIZE as i64;
        let images = Tensor::from_array((vec![1, 3, n, n], chw(&p.rgba, true))).map_err(ort_err)?;
        let sizes = Tensor::from_array((vec![1, 2], vec![n, n])).map_err(ort_err)?;
        let mut session = self
            .bubble_text
            .lock()
            .map_err(|_| Error::Other("onnx: session poisoned".into()))?;
        let out = session
            .run(ort::inputs!["images" => images, "orig_target_sizes" => sizes])
            .map_err(ort_err)?;
        let (_, labels) = out["labels"].try_extract_tensor::<i64>().map_err(ort_err)?;
        let (_, boxes) = out["boxes"].try_extract_tensor::<f32>().map_err(ort_err)?;
        let (_, scores) = out["scores"].try_extract_tensor::<f32>().map_err(ort_err)?;
        Ok(rtdetr_detections(labels, boxes, scores, p))
    }

    fn run_face(&self, p: &Prepared) -> Result<Vec<Detection>> {
        let n = INPUT_SIZE as i64;
        let images =
            Tensor::from_array((vec![1, 3, n, n], chw(&p.rgba, false))).map_err(ort_err)?;
        let mut session = self
            .face
            .lock()
            .map_err(|_| Error::Other("onnx: session poisoned".into()))?;
        let out = session
            .run(ort::inputs!["images" => images])
            .map_err(ort_err)?;
        let (shape, data) = out[0].try_extract_tensor::<f32>().map_err(ort_err)?;
        let dims: Vec<usize> = shape.iter().map(|&d| d.max(0) as usize).collect();
        Ok(yolo_detections(&dims, data, p))
    }
}

/// RT-DETR outputs: labels 0 = bubble, 1 = text in bubble, 2 = free text; boxes xyxy in input px.
fn rtdetr_detections(
    labels: &[i64],
    boxes: &[f32],
    scores: &[f32],
    p: &Prepared,
) -> Vec<Detection> {
    labels
        .iter()
        .zip(scores)
        .zip(boxes.as_chunks::<4>().0.iter())
        .filter(|((_, &s), _)| s >= DETECTOR_SCORE_THRESHOLD)
        .filter_map(|((&l, &s), b)| {
            let class = match l {
                0 => Class::Bubble,
                1 | 2 => Class::Text,
                _ => return None,
            };
            Some(Detection {
                class,
                score: s,
                x0: b[0] * p.scale_x,
                y0: b[1] * p.scale_y,
                x1: b[2] * p.scale_x,
                y1: b[3] * p.scale_y,
            })
        })
        .collect()
}

/// YOLOv8-style output `[1, 4 + classes, N]` (or transposed): cx, cy, w, h, class scores.
fn yolo_detections(dims: &[usize], data: &[f32], p: &Prepared) -> Vec<Detection> {
    let (attrs, count, transposed) = match dims {
        [_, a, n] if (5..=16).contains(a) => (*a, *n, false),
        [_, n, a] if (5..=16).contains(a) => (*a, *n, true),
        _ => return Vec::new(),
    };
    let at = |i: usize, k: usize| {
        if transposed {
            data[i * attrs + k]
        } else {
            data[k * count + i]
        }
    };
    (0..count)
        .filter_map(|i| {
            let score = (4..attrs).map(|k| at(i, k)).fold(0f32, f32::max);
            if score < DETECTOR_SCORE_THRESHOLD {
                return None;
            }
            let (cx, cy, w, h) = (at(i, 0), at(i, 1), at(i, 2), at(i, 3));
            Some(Detection {
                class: Class::Face,
                score,
                x0: (cx - w / 2.0) * p.scale_x,
                y0: (cy - h / 2.0) * p.scale_y,
                x1: (cx + w / 2.0) * p.scale_x,
                y1: (cy + h / 2.0) * p.scale_y,
            })
        })
        .collect()
}
