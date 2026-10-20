//! Typeset-text detection: clusters of letter-sized high-contrast blobs on a flat background.

use super::components::{self, PixelBox};
use super::morphology;
use super::raster::RgbStrip;

const TEXT_CONTRAST_THR: i32 = 25;
const TEXT_BLUR_RADIUS: usize = 15; // 31x31 box blur
/// Glyph sizes in analysis pixels; shouted bubble lettering on webtoons reaches ~60 rows.
const GLYPH_H: std::ops::RangeInclusive<usize> = 5..=64;
const GLYPH_W: std::ops::RangeInclusive<usize> = 2..=80;

/// `(2r+1)^2` box mean of one channel, border-clamped like `cv2.blur`.
pub fn box_blur_channel(s: &RgbStrip, c: usize, r: usize) -> Vec<u8> {
    let (w, h) = (s.width, s.height);
    let clamp = |v: isize, hi: usize| v.clamp(0, hi as isize - 1) as usize;
    let mut tmp = vec![0i32; w * h];
    for y in 0..h {
        let row = s.row(y);
        let at = |x: isize| row[clamp(x, w) * 3 + c] as i32;
        let mut acc: i32 = (-(r as isize)..=r as isize).map(at).sum();
        for x in 0..w {
            tmp[y * w + x] = acc;
            acc -= at(x as isize - r as isize);
            acc += at(x as isize + r as isize + 1);
        }
    }
    let n = ((2 * r + 1) * (2 * r + 1)) as i32;
    let mut out = vec![0u8; w * h];
    for x in 0..w {
        let at = |y: isize| tmp[clamp(y, h) * w + x];
        let mut acc: i32 = (-(r as isize)..=r as isize).map(at).sum();
        for y in 0..h {
            out[y * w + x] = ((acc + n / 2) / n) as u8;
            acc -= at(y as isize - r as isize);
            acc += at(y as isize + r as isize + 1);
        }
    }
    out
}

fn contrast_masks(s: &RgbStrip) -> (Vec<u8>, Vec<u8>) {
    let n = s.width * s.height;
    let (mut neg, mut pos) = (vec![0u8; n], vec![0u8; n]);
    for c in 0..3 {
        let blur = box_blur_channel(s, c, TEXT_BLUR_RADIUS);
        for i in 0..n {
            let d = s.pix[i * 3 + c] as i32 - blur[i] as i32;
            if d > TEXT_CONTRAST_THR {
                pos[i] = 1;
            } else if -d > TEXT_CONTRAST_THR {
                neg[i] = 1;
            }
        }
    }
    (neg, pos)
}

fn letter_mask(binm: &[u8], w: usize, h: usize) -> Vec<u8> {
    let (_, comps) = components::label(binm, w, h);
    let mut letters = vec![0u8; w * h];
    for c in comps {
        let (cw, ch) = (c.bbox.width(), c.bbox.height());
        let letter_sized = GLYPH_H.contains(&ch) && GLYPH_W.contains(&cw);
        if letter_sized && c.area >= 8 && (c.area as f64) <= 0.9 * (cw * ch) as f64 {
            for y in c.bbox.y0..c.bbox.y1 {
                letters[y * w + c.bbox.x0..y * w + c.bbox.x1].fill(1);
            }
        }
    }
    letters
}

pub fn text_boxes(s: &RgbStrip) -> Vec<PixelBox> {
    let (w, h) = (s.width, s.height);
    let (neg, pos) = contrast_masks(s);
    let mut out = Vec::new();
    for binm in [&neg, &pos] {
        let letters = letter_mask(binm, w, h);
        let blocks = morphology::dilate(&letters, w, h, 8, 4);
        let (_, bcomps) = components::label(&blocks, w, h);
        for b in bcomps {
            let (bw, bh) = (b.bbox.width(), b.bbox.height());
            if !(bw >= 20 && bh >= 10 && (bw as f64) < 0.8 * w as f64 && bh < 400) {
                continue;
            }
            if qualify_text_block(s, binm, &letters, b.bbox) {
                out.push(b.bbox);
            }
        }
    }
    out
}

fn ink_runs(binm: &[u8], w: usize, b: PixelBox) -> usize {
    let mut runs = 0;
    let mut prev = false;
    for x in b.x0..b.x1 {
        let ink = (b.y0..b.y1).any(|y| binm[y * w + x] != 0);
        if ink && !prev {
            runs += 1;
        }
        prev = ink;
    }
    runs
}

fn glyph_heights_uniform(glyphs: &[components::Component]) -> bool {
    let n = glyphs.len() as f64;
    let (sum, sq) = glyphs.iter().fold((0.0, 0.0), |(s, q), g| {
        let hh = g.bbox.height() as f64;
        (s + hh, q + hh * hh)
    });
    let mean = sum / n;
    let std = (sq / n - mean * mean).max(0.0).sqrt();
    std <= 0.6 * mean
}

fn flat_background(s: &RgbStrip, sub: &[u8], b: PixelBox, limit: f64) -> bool {
    let (bw, bh) = (b.width(), b.height());
    let grown = morphology::dilate(sub, bw, bh, 2, 2);
    let (mut cnt, mut sum, mut sq) = (0usize, [0f64; 3], [0f64; 3]);
    for y in 0..bh {
        for x in 0..bw {
            if grown[y * bw + x] != 0 {
                continue;
            }
            let p = s.px(b.x0 + x, b.y0 + y);
            for c in 0..3 {
                sum[c] += p[c] as f64;
                sq[c] += (p[c] as f64) * (p[c] as f64);
            }
            cnt += 1;
        }
    }
    if cnt < 50 {
        return false;
    }
    (0..3).all(|c| {
        let m = sum[c] / cnt as f64;
        (sq[c] / cnt as f64 - m * m).max(0.0).sqrt() <= limit
    })
}

fn qualify_text_block(s: &RgbStrip, binm: &[u8], letters: &[u8], b: PixelBox) -> bool {
    const MIN_GLYPHS: usize = 3;
    let (w, bw, bh) = (s.width, b.width(), b.height());
    let sub: Vec<u8> = (0..bh)
        .flat_map(|y| {
            letters[(b.y0 + y) * w + b.x0..(b.y0 + y) * w + b.x1]
                .iter()
                .copied()
        })
        .collect();
    let (_, glyphs) = components::label(&sub, bw, bh);
    let line_like = bh <= 40 && bw as f64 >= 2.5 * bh as f64 && ink_runs(binm, w, b) >= 3;
    if glyphs.len() < MIN_GLYPHS && !line_like {
        return false;
    }
    if glyphs.len() >= 2 && !line_like && !glyph_heights_uniform(&glyphs) {
        return false;
    }
    flat_background(s, &sub, b, if line_like { 40.0 } else { 26.0 })
}
