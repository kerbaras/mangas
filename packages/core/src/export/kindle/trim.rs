//! Margin trim: blank borders found by scanning lines in from each edge. A line is blank when
//! nearly all its pixels are close to the edge colour. Pure functions over decoded images.

use image::{Rgba, RgbaImage};

/// Per-channel distance still counted as the border colour (JPEG noise, paper tone).
const TOLERANCE: i32 = 24;
/// Share of a line's pixels that may differ (dust, scan specks).
const NOISE: f64 = 0.005;

fn close(p: &Rgba<u8>, bg: &Rgba<u8>) -> bool {
    (0..3).all(|c| (p[c] as i32 - bg[c] as i32).abs() <= TOLERANCE)
}

fn blank(pixels: impl Iterator<Item = Rgba<u8>>, len: u32, bg: &Rgba<u8>) -> bool {
    let limit = (len as f64 * NOISE).floor() as usize;
    pixels.filter(|p| !close(p, bg)).take(limit + 1).count() <= limit
}

fn column_blank(img: &RgbaImage, x: u32, bg: &Rgba<u8>) -> bool {
    blank(
        (0..img.height()).map(|y| *img.get_pixel(x, y)),
        img.height(),
        bg,
    )
}

fn row_blank(img: &RgbaImage, y: u32, (x0, x1): (u32, u32), bg: &Rgba<u8>) -> bool {
    blank((x0..x1).map(|x| *img.get_pixel(x, y)), x1 - x0, bg)
}

/// Blank columns on the left and right, each at most a quarter of the width. `None` when the
/// image is blank all the way across, so it says nothing about where the art is.
pub fn side_margins(img: &RgbaImage) -> Option<(u32, u32)> {
    let (w, h) = img.dimensions();
    if w < 4 || h == 0 {
        return Some((0, 0));
    }
    let (left_bg, right_bg) = (*img.get_pixel(0, 0), *img.get_pixel(w - 1, 0));
    let left = (0..w)
        .take_while(|&x| column_blank(img, x, &left_bg))
        .count() as u32;
    if left == w {
        return None;
    }
    let right = (0..w)
        .rev()
        .take_while(|&x| column_blank(img, x, &right_bg))
        .count() as u32;
    Some((left.min(w / 4), right.min(w / 4)))
}

/// The smaller margins on each side: what is blank in both.
pub fn narrower(a: Option<(u32, u32)>, b: Option<(u32, u32)>) -> Option<(u32, u32)> {
    match (a, b) {
        (Some((l1, r1)), Some((l2, r2))) => Some((l1.min(l2), r1.min(r2))),
        (one, None) | (None, one) => one,
    }
}

/// The image without `left`/`right` columns.
pub fn crop_sides(img: &RgbaImage, (left, right): (u32, u32)) -> RgbaImage {
    if left == 0 && right == 0 {
        return img.clone();
    }
    let w = img.width().saturating_sub(left + right).max(1);
    image::imageops::crop_imm(img, left, 0, w, img.height()).to_image()
}

/// A page with its blank borders cut off, each side at most a quarter of the page.
pub fn trim_page(img: &RgbaImage) -> RgbaImage {
    match trim_box(img) {
        Some((x, y, w, h)) => image::imageops::crop_imm(img, x, y, w, h).to_image(),
        None => img.clone(),
    }
}

/// `trim_page` without a copy when there is nothing to trim.
pub fn trim_owned(img: RgbaImage) -> RgbaImage {
    match trim_box(&img) {
        Some(b) if b != (0, 0, img.width(), img.height()) => crop_owned(img, b),
        _ => img,
    }
}

/// `crop_sides` on an owned page, cropped in its own buffer.
pub fn crop_sides_owned(img: RgbaImage, (left, right): (u32, u32)) -> RgbaImage {
    if left == 0 && right == 0 {
        return img;
    }
    let w = img.width().saturating_sub(left + right).max(1);
    let h = img.height();
    crop_owned(img, (left, 0, w, h))
}

/// The `w x h` box at `(x, y)` (inside `img`), moved row by row to the front of `img`'s own
/// buffer: the same pixels as `crop_imm(..).to_image()` without a second page in memory.
fn crop_owned(img: RgbaImage, (x, y, w, h): (u32, u32, u32, u32)) -> RgbaImage {
    let stride = img.width() as usize * 4;
    let (row, left) = (w as usize * 4, x as usize * 4);
    let mut buf = img.into_raw();
    for r in 0..h as usize {
        let from = (y as usize + r) * stride + left;
        buf.copy_within(from..from + row, r * row);
    }
    buf.truncate(row * h as usize);
    RgbaImage::from_raw(w, h, buf).unwrap_or_else(|| RgbaImage::new(w, h))
}

/// The page with its blank margins cut off (a quarter of the height at most per side).
fn trim_box(img: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (left, right) = side_margins(img)?;
    let (w, h) = img.dimensions();
    let cols = (left, w - right);
    let (top_bg, bottom_bg) = (*img.get_pixel(left, 0), *img.get_pixel(left, h - 1));
    let top = (0..h)
        .take_while(|&y| row_blank(img, y, cols, &top_bg))
        .count() as u32;
    let bottom = (0..h)
        .rev()
        .take_while(|&y| row_blank(img, y, cols, &bottom_bg))
        .count() as u32;
    let (top, bottom) = (top.min(h / 4), bottom.min(h / 4));
    Some((left, top, w - left - right, h - top - bottom))
}
