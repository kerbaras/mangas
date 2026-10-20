//! Flood fill that grows a detected text block to the flat-colour speech bubble around it.

use super::components::PixelBox;
use super::raster::RgbStrip;

/// The fill stops inside the outline; the box takes the outline too.
const OUTLINE: usize = 4;
/// How far above / below the text a leaked balloon's outline is looked for.
const LEAK_REACH: usize = 300;

/// Grows a text block to the flat-colour balloon enclosing it (fixed-range flood fill from the
/// balloon colour just above the text, so anti-aliased outlines do not leak).
pub fn bubble_around(s: &RgbStrip, t: PixelBox) -> PixelBox {
    const TOL: i32 = 20;
    const MAX_H: usize = 900;
    const MAX_W_FRAC: f64 = 0.9;
    let w = s.width;
    let (wy0, wy1) = (t.y0.saturating_sub(MAX_H), (t.y1 + MAX_H).min(s.height));
    let wh = wy1 - wy0;
    let seed_x = (t.x0 + t.x1) / 2;
    let seed_y = t.y0.saturating_sub(3) - wy0;
    let seed = s.px(seed_x, wy0 + seed_y);
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
        let neighbours = [
            (x + 1 < w).then(|| i + 1),
            (x > 0).then(|| i - 1),
            (y + 1 < wh).then(|| i + w),
            (y > 0).then(|| i - w),
        ];
        for j in neighbours.into_iter().flatten() {
            if visited[j] == 0 && close(seed, s.px(j % w, wy0 + j / w)) {
                visited[j] = 1;
                stack.push(j);
            }
        }
    }
    let (by0, by1) = (by0 + wy0, by1 + wy0);
    if (bx1 - bx0) as f64 > MAX_W_FRAC * w as f64 || by1 - by0 > MAX_H {
        return leaked(s, t, |p| close(seed, p));
    }
    PixelBox {
        x0: bx0.min(t.x0).saturating_sub(OUTLINE),
        y0: by0.min(t.y0).saturating_sub(OUTLINE),
        x1: (bx1.max(t.x1) + OUTLINE).min(w),
        y1: (by1.max(t.y1) + OUTLINE).min(s.height),
    }
}

/// A balloon whose fill leaked into a background of its own colour (a white balloon touching a
/// white gutter): its outline is still found straight above and below the text. The box spans
/// the farthest outline hit over the text's columns (a tail reaches higher than the arc); columns
/// with no outline within reach are ignored, and with none at all the text box is padded.
fn leaked(s: &RgbStrip, t: PixelBox, inside: impl Fn([u8; 3]) -> bool) -> PixelBox {
    const PAD: usize = 10;
    const STEP: usize = 4;
    let cols = || (t.x0..t.x1).step_by(STEP);
    let inside = &inside;
    let edge = |x: usize| move |y: &usize| !inside(s.px(x, *y));
    let up = cols()
        .filter_map(|x| (t.y0.saturating_sub(LEAK_REACH)..t.y0).rev().find(edge(x)))
        .min();
    let down = cols()
        .filter_map(|x| (t.y1..(t.y1 + LEAK_REACH).min(s.height)).find(edge(x)))
        .max();
    PixelBox {
        x0: t.x0.saturating_sub(PAD),
        y0: up.map_or(t.y0.saturating_sub(PAD), |y| y.saturating_sub(OUTLINE)),
        x1: (t.x1 + PAD).min(s.width),
        y1: down.map_or(t.y1 + PAD, |y| y + 1 + OUTLINE).min(s.height),
    }
}
