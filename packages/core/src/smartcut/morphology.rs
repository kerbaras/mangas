//! Separable binary morphology on `u8` masks (0 / 1), O(N) per pass.

fn dilate_1d(input: impl Fn(usize) -> u8, out: &mut dyn FnMut(usize, u8), n: usize, radius: usize) {
    let mut cnt = 0usize;
    for i in 0..=radius.min(n.saturating_sub(1)) {
        if input(i) != 0 {
            cnt += 1;
        }
    }
    for i in 0..n {
        out(i, u8::from(cnt > 0));
        if i >= radius && input(i - radius) != 0 {
            cnt -= 1;
        }
        let hi = i + radius + 1;
        if hi < n && input(hi) != 0 {
            cnt += 1;
        }
    }
}

/// `out[p] = any(m[q])` over the `(2rx+1) x (2ry+1)` window around `p`.
pub fn dilate(m: &[u8], w: usize, h: usize, rx: usize, ry: usize) -> Vec<u8> {
    let mut tmp = vec![0u8; m.len()];
    for y in 0..h {
        let row = &m[y * w..(y + 1) * w];
        let mut set = |i: usize, v: u8| tmp[y * w + i] = v;
        dilate_1d(|i| row[i], &mut set, w, rx);
    }
    let mut out = vec![0u8; m.len()];
    for x in 0..w {
        let mut set = |i: usize, v: u8| out[i * w + x] = v;
        dilate_1d(|i| tmp[i * w + x], &mut set, h, ry);
    }
    out
}

pub fn invert(m: &[u8]) -> Vec<u8> {
    m.iter().map(|&v| u8::from(v == 0)).collect()
}

/// Dilation followed by erosion. Erosion is clamped at the border so shapes touching the edge
/// are not eaten from outside.
pub fn close(m: &[u8], w: usize, h: usize, r: usize) -> Vec<u8> {
    let d = dilate(m, w, h, r, r);
    let mut out = invert(&dilate(&invert(&d), w, h, r, r));
    for y in 0..h {
        for x in 0..w {
            let border = x < r || y < r || x + r >= w || y + r >= h;
            if border && d[y * w + x] != 0 {
                out[y * w + x] = 1;
            }
        }
    }
    out
}
