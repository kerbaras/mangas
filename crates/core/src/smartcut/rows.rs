//! Per-row scalar statistics: uniformity (gutter detection), texture and box coverage.

use super::components::PixelBox;
use super::raster::RgbStrip;

/// Largest per-channel standard deviation of each row.
pub fn row_uniformity(s: &RgbStrip) -> Vec<f64> {
    let n = s.width as f64;
    (0..s.height)
        .map(|y| {
            let (mut sum, mut sq) = ([0f64; 3], [0f64; 3]);
            for p in s.row(y).chunks_exact(3) {
                for c in 0..3 {
                    let v = p[c] as f64;
                    sum[c] += v;
                    sq[c] += v * v;
                }
            }
            (0..3)
                .map(|c| {
                    let m = sum[c] / n;
                    (sq[c] / n - m * m).max(0.0).sqrt()
                })
                .fold(0.0, f64::max)
        })
        .collect()
}

#[inline]
pub fn luma(p: &[u8]) -> i32 {
    (299 * p[0] as i32 + 587 * p[1] as i32 + 114 * p[2] as i32) / 1000
}

/// Mean absolute horizontal luma gradient per row, normalised so the mean over rows is 1.
pub fn row_texture(s: &RgbStrip) -> Vec<f32> {
    let mut out: Vec<f32> = (0..s.height)
        .map(|y| {
            let row = s.row(y);
            if s.width <= 1 {
                return 0.0;
            }
            let acc: i32 = row
                .chunks_exact(3)
                .map(luma)
                .collect::<Vec<_>>()
                .windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .sum();
            acc as f32 / (s.width - 1) as f32
        })
        .collect();
    let mean = out.iter().map(|&v| v as f64).sum::<f64>() / s.height.max(1) as f64 + 1e-6;
    out.iter_mut().for_each(|v| *v /= mean as f32);
    out
}

pub fn rows_covered(boxes: &[PixelBox], h: usize) -> Vec<f32> {
    let mut r = vec![0f32; h];
    for b in boxes {
        r[b.y0.min(h)..b.y1.min(h)]
            .iter_mut()
            .for_each(|v| *v = 1.0);
    }
    r
}
