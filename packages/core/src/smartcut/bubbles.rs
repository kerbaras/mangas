//! Flood fill that grows a detected text block to the flat-colour speech bubble around it.

use super::components::PixelBox;
use super::raster::RgbStrip;

/// Grows a text block to the flat-colour balloon enclosing it (floating-range flood fill).
pub fn bubble_around(s: &RgbStrip, t: PixelBox) -> PixelBox {
    const TOL: i32 = 20;
    const MAX_H: usize = 600;
    const MAX_W_FRAC: f64 = 0.7;
    let w = s.width;
    let (wy0, wy1) = (t.y0.saturating_sub(MAX_H), (t.y1 + MAX_H).min(s.height));
    let wh = wy1 - wy0;
    let seed_x = (t.x0 + t.x1) / 2;
    let seed_y = t.y0.saturating_sub(3) - wy0;
    let mut visited = vec![0u8; w * wh];
    let mut stack = vec![seed_y * w + seed_x];
    visited[stack[0]] = 1;
    let (mut bx0, mut bx1, mut by0, mut by1) = (seed_x, seed_x + 1, seed_y, seed_y + 1);
    let close = |p: [u8; 3], q: [u8; 3]| (0..3).all(|c| (p[c] as i32 - q[c] as i32).abs() <= TOL);
    while let Some(i) = stack.pop() {
        let (y, x) = (i / w, i % w);
        bx0 = bx0.min(x);
        bx1 = bx1.max(x + 1);
        by0 = by0.min(y);
        by1 = by1.max(y + 1);
        let p = s.px(x, wy0 + y);
        let neighbours = [
            (x + 1 < w).then(|| i + 1),
            (x > 0).then(|| i - 1),
            (y + 1 < wh).then(|| i + w),
            (y > 0).then(|| i - w),
        ];
        for j in neighbours.into_iter().flatten() {
            if visited[j] == 0 && close(p, s.px(j % w, wy0 + j / w)) {
                visited[j] = 1;
                stack.push(j);
            }
        }
    }
    let (by0, by1) = (by0 + wy0, by1 + wy0);
    if (bx1 - bx0) as f64 > MAX_W_FRAC * w as f64 || by1 - by0 > MAX_H {
        // leaked into the background: just pad the text box a little
        return PixelBox {
            x0: t.x0.saturating_sub(10),
            y0: t.y0.saturating_sub(10),
            x1: (t.x1 + 10).min(w),
            y1: (t.y1 + 10).min(s.height),
        };
    }
    PixelBox {
        x0: bx0.min(t.x0),
        y0: by0.min(t.y0),
        x1: bx1.max(t.x1),
        y1: by1.max(t.y1),
    }
}
