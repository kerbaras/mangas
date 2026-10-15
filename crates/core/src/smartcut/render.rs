//! Draws a row range of a strip onto a device-sized page: scaled to the device width (or a
//! little smaller when the page was allowed to shrink), top-aligned, letterboxed in `bg`.

use image::{Rgba, RgbaImage};

use super::paginate::PageSpan;
use super::raster::{crop_rows, resize_rgba};
use crate::domain::Result;

pub fn render_page(
    src: &RgbaImage,
    span: PageSpan,
    bg: [u8; 3],
    dev_w: u32,
    dev_h: u32,
) -> Result<RgbaImage> {
    let mut dst = RgbaImage::from_pixel(dev_w, dev_h, Rgba([bg[0], bg[1], bg[2], 255]));
    let src_w = src.width();
    let src_h = span.end.saturating_sub(span.start) as u32;
    if src_h == 0 || src_w == 0 || dev_w == 0 || dev_h == 0 {
        return Ok(dst);
    }
    let mut scale = dev_w as f64 / src_w as f64;
    let (mut out_w, mut out_h) = (dev_w, (src_h as f64 * scale).round() as u32);
    if out_h > dev_h {
        scale = dev_h as f64 / src_h as f64;
        out_w = (src_w as f64 * scale).round() as u32;
        out_h = dev_h;
    }
    let rows = crop_rows(src, span.start as u32, span.end as u32);
    let scaled = resize_rgba(&rows, out_w.max(1), out_h.max(1))?;
    let x0 = (dev_w - out_w.min(dev_w)) / 2;
    image::imageops::replace(&mut dst, &scaled, x0 as i64, 0);
    Ok(dst)
}
