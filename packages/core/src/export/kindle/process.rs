//! Per-page processing for a device: fit, grayscale, tone, sharpen, JPEG.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, GrayImage, ImageEncoder, RgbaImage};

use super::filters::{
    apply_lut, flatten_on_white, grayscale_rgba, is_identity, sharpen_gray, tone_lut,
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
    let flat = flatten_on_white(fitted.as_raw());
    if s.grayscale {
        let mut gray = grayscale_rgba(&flat);
        let lut = tone_lut(s.contrast, s.gamma);
        if !is_identity(&lut) {
            gray = apply_lut(&gray, &lut);
        }
        if s.sharpen {
            gray = sharpen_gray(&gray, w as usize, h as usize);
        }
        return Ok(EncodedPage {
            jpeg: encode_jpeg(&gray, w, h, ExtendedColorType::L8, s.quality)?,
            width: w,
            height: h,
        });
    }
    let lut = tone_lut(s.contrast, s.gamma);
    let rgb: Vec<u8> = flat
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [lut[p[0] as usize], lut[p[1] as usize], lut[p[2] as usize]])
        .collect();
    Ok(EncodedPage {
        jpeg: encode_jpeg(&rgb, w, h, ExtendedColorType::Rgb8, s.quality)?,
        width: w,
        height: h,
    })
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
