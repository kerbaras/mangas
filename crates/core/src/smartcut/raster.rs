//! Packed 8-bit RGB raster used by the analysis, plus the few whole-image operations the
//! paginator needs (downscale for analysis, stack chunks, crop rows).

use fast_image_resize::{images::Image as FirImage, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::RgbaImage;

use crate::domain::{Error, Result};

/// Row-major RGB pixels, `width * height * 3` bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbStrip {
    pub width: usize,
    pub height: usize,
    pub pix: Vec<u8>,
}

impl RgbStrip {
    pub fn from_rgba(img: &RgbaImage) -> Self {
        let (width, height) = (img.width() as usize, img.height() as usize);
        let pix = img
            .as_raw()
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        RgbStrip { width, height, pix }
    }

    #[inline]
    pub fn row(&self, y: usize) -> &[u8] {
        &self.pix[y * self.width * 3..(y + 1) * self.width * 3]
    }

    #[inline]
    pub fn px(&self, x: usize, y: usize) -> [u8; 3] {
        let i = (y * self.width + x) * 3;
        [self.pix[i], self.pix[i + 1], self.pix[i + 2]]
    }

    /// The leftmost `cols` columns as their own strip.
    pub fn left_columns(&self, cols: usize) -> RgbStrip {
        let w = cols.min(self.width);
        let pix = (0..self.height)
            .flat_map(|y| self.row(y)[..w * 3].iter().copied())
            .collect();
        RgbStrip {
            width: w,
            height: self.height,
            pix,
        }
    }
}

/// Lanczos3 resize of an RGBA image to exactly `w x h`.
pub fn resize_rgba(src: &RgbaImage, w: u32, h: u32) -> Result<RgbaImage> {
    let src_view = FirImage::from_vec_u8(
        src.width(),
        src.height(),
        src.as_raw().clone(),
        PixelType::U8x4,
    )
    .map_err(|e| Error::Other(format!("resize: {e}")))?;
    let mut dst = FirImage::new(w, h, PixelType::U8x4);
    let opts = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(
        fast_image_resize::FilterType::Lanczos3,
    ));
    Resizer::new()
        .resize(&src_view, &mut dst, &opts)
        .map_err(|e| Error::Other(format!("resize: {e}")))?;
    RgbaImage::from_raw(w, h, dst.into_vec())
        .ok_or_else(|| Error::Other("resize: bad buffer".into()))
}

/// A copy no wider than `max_w` (aspect preserved); the input itself when already narrow enough.
pub fn resized_for_analysis(img: &RgbaImage, max_w: u32) -> Result<RgbaImage> {
    if img.width() <= max_w {
        return Ok(img.clone());
    }
    let h = ((img.height() as f64 * max_w as f64 / img.width() as f64).round() as u32).max(1);
    resize_rgba(img, max_w, h)
}

/// `top` (if any) stacked above `bottom`; both must share a width.
pub fn stack(top: Option<&RgbaImage>, bottom: &RgbaImage) -> RgbaImage {
    let Some(top) = top else {
        return bottom.clone();
    };
    let mut buf = Vec::with_capacity(top.as_raw().len() + bottom.as_raw().len());
    buf.extend_from_slice(top.as_raw());
    buf.extend_from_slice(bottom.as_raw());
    RgbaImage::from_raw(bottom.width(), top.height() + bottom.height(), buf)
        .unwrap_or_else(|| bottom.clone())
}

/// Rows `[y0, y1)` of `img` as a new image.
pub fn crop_rows(img: &RgbaImage, y0: u32, y1: u32) -> RgbaImage {
    let stride = img.width() as usize * 4;
    let (a, b) = (y0.min(img.height()) as usize, y1.min(img.height()) as usize);
    let buf = img.as_raw()[a * stride..b.max(a) * stride].to_vec();
    RgbaImage::from_raw(img.width(), (b.max(a) - a) as u32, buf)
        .unwrap_or_else(|| RgbaImage::new(img.width(), 0))
}
