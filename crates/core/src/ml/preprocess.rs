//! Image -> model input tensors. Both detectors take a 640x640 RGB tile.

use image::RgbaImage;

use crate::domain::Result;
use crate::smartcut::raster::resize_rgba;

pub const INPUT_SIZE: u32 = 640;

const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];

/// A tile resized to the model input, plus the factors that map model coordinates back.
pub struct Prepared {
    pub rgba: RgbaImage,
    pub scale_x: f32,
    pub scale_y: f32,
}

pub fn prepare(tile: &RgbaImage) -> Result<Prepared> {
    let rgba = if tile.dimensions() == (INPUT_SIZE, INPUT_SIZE) {
        tile.clone()
    } else {
        resize_rgba(tile, INPUT_SIZE, INPUT_SIZE)?
    };
    Ok(Prepared {
        rgba,
        scale_x: tile.width() as f32 / INPUT_SIZE as f32,
        scale_y: tile.height() as f32 / INPUT_SIZE as f32,
    })
}

/// Planar CHW float tensor in 0..1, optionally ImageNet-normalised.
pub fn chw(img: &RgbaImage, normalise: bool) -> Vec<f32> {
    let n = (img.width() * img.height()) as usize;
    let mut out = vec![0f32; 3 * n];
    for (i, p) in img.as_raw().chunks_exact(4).enumerate() {
        for c in 0..3 {
            let v = p[c] as f32 / 255.0;
            out[c * n + i] = if normalise { (v - MEAN[c]) / STD[c] } else { v };
        }
    }
    out
}
