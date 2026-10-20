//! Per-page processing for a device: fit, grayscale, tone, sharpen, JPEG.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, GrayImage, ImageEncoder, RgbaImage};

use super::filters::{
    apply_lut, flatten_on_white_in_place, grayscale_rgba, is_identity, sharpen_gray, tone_lut,
};
use super::Device;
use crate::domain::{Error, Result};
use crate::smartcut::raster::resize_rgba;

/// How pages are filtered and encoded for one device.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageSettings {
    pub max_width: u32,
    pub max_height: u32,
    pub quality: u8,
    pub grayscale: bool,
    pub sharpen: bool,
    pub contrast: f64,
    pub gamma: f64,
}

impl ImageSettings {
    /// Recommended settings for a device: higher JPEG quality at 300 dpi; e-ink gets a slight
    /// contrast boost, gamma 0.9 and sharpening.
    pub fn for_device(d: &Device) -> Self {
        ImageSettings {
            max_width: d.width,
            max_height: d.height,
            quality: crate::export::page::default_quality(d.dpi),
            grayscale: d.grayscale,
            sharpen: d.grayscale,
            contrast: 1.1,
            gamma: if d.grayscale { 0.9 } else { 1.0 },
        }
    }
}

/// Encoded page ready for the EPUB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedPage {
    pub jpeg: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Largest size within the device bounds that keeps the aspect ratio (never upscales).
pub fn fit_dimensions(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    if w <= max_w && h <= max_h {
        return (w, h);
    }
    let scale = (max_w as f64 / w as f64).min(max_h as f64 / h as f64);
    (
        ((w as f64 * scale) as u32).max(1),
        ((h as f64 * scale) as u32).max(1),
    )
}

/// Runs the whole chain on a decoded page.
pub fn process_page(img: &RgbaImage, s: &ImageSettings) -> Result<EncodedPage> {
    let (w, h) = fit_dimensions(img.width(), img.height(), s.max_width, s.max_height);
    let fitted = if (w, h) != (img.width(), img.height()) {
        resize_rgba(img, w, h)?
    } else {
        img.clone()
    };
    finish(fitted, s)
}

/// [`process_page`] on an owned page, dropped as soon as it is resized: a page worker holds one
/// full-size page at most.
pub fn process_owned(img: RgbaImage, s: &ImageSettings) -> Result<EncodedPage> {
    let (w, h) = fit_dimensions(img.width(), img.height(), s.max_width, s.max_height);
    let fitted = if (w, h) != (img.width(), img.height()) {
        resize_rgba(&img, w, h)?
    } else {
        img
    };
    finish(fitted, s)
}

/// Flattens, tones and encodes a fitted page, reusing its buffer.
fn finish(fitted: RgbaImage, s: &ImageSettings) -> Result<EncodedPage> {
    let (w, h) = fitted.dimensions();
    let mut flat = fitted.into_raw();
    flatten_on_white_in_place(&mut flat);
    let lut = tone_lut(s.contrast, s.gamma);
    let (buf, color) = if s.grayscale {
        let mut gray = grayscale_rgba(&flat);
        drop(flat);
        if !is_identity(&lut) {
            gray = apply_lut(&gray, &lut);
        }
        if s.sharpen {
            gray = sharpen_gray(&gray, w as usize, h as usize);
        }
        (gray, ExtendedColorType::L8)
    } else {
        (toned_rgb(flat, &lut), ExtendedColorType::Rgb8)
    };
    Ok(EncodedPage {
        jpeg: encode_jpeg(&buf, w, h, color, s.quality)?,
        width: w,
        height: h,
    })
}

/// RGB with `lut` on every channel, packed into the front of the flattened RGBA buffer.
fn toned_rgb(mut rgba: Vec<u8>, lut: &[u8; 256]) -> Vec<u8> {
    let n = rgba.len() / 4;
    for i in 0..n {
        let [r, g, b] = [rgba[4 * i], rgba[4 * i + 1], rgba[4 * i + 2]];
        rgba[3 * i..3 * i + 3].copy_from_slice(&[
            lut[r as usize],
            lut[g as usize],
            lut[b as usize],
        ]);
    }
    rgba.truncate(3 * n);
    rgba
}

pub fn encode_jpeg(
    buf: &[u8],
    w: u32,
    h: u32,
    color: ExtendedColorType,
    quality: u8,
) -> Result<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    JpegEncoder::new_with_quality(&mut out, quality).write_image(buf, w, h, color)?;
    Ok(out.into_inner())
}

/// Convenience for tests and previews: the grayscale pipeline as an image.
pub fn to_gray_image(img: &RgbaImage, s: &ImageSettings) -> Result<GrayImage> {
    let mut gray = grayscale_rgba(img.as_raw());
    gray = apply_lut(&gray, &tone_lut(s.contrast, s.gamma));
    if s.sharpen {
        gray = sharpen_gray(&gray, img.width() as usize, img.height() as usize);
    }
    GrayImage::from_raw(img.width(), img.height(), gray)
        .ok_or_else(|| Error::Other("gray buffer".into()))
}

/// `process_page` as a page of the book.
pub fn encode(img: &RgbaImage, settings: &ImageSettings) -> Result<crate::export::epub::PageImage> {
    process_page(img, settings).map(page_image)
}

/// [`encode`] on an owned page (see [`process_owned`]).
pub fn encode_owned(
    img: RgbaImage,
    settings: &ImageSettings,
) -> Result<crate::export::epub::PageImage> {
    process_owned(img, settings).map(page_image)
}

fn page_image(e: EncodedPage) -> crate::export::epub::PageImage {
    crate::export::epub::PageImage {
        jpeg: e.jpeg,
        width: e.width,
        height: e.height,
    }
}
