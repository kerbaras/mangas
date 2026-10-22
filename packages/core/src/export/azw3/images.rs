//! The standard (low-definition) image records. Kindle firmware reads at most 128 KiB per image
//! record (Amazon Kindle Publishing Guidelines, "image size"), so a page JPEG over that gets a
//! smaller copy here and keeps its original in the HD container; a page under it is stored as
//! is. Copies are re-encoded at falling quality first, then scaled down, deterministically.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageEncoder, ImageFormat};

use crate::domain::{Error, Result};

/// The largest standard image record.
pub const LIMIT: usize = 128 * 1024;
const QUALITIES: [u8; 4] = [80, 70, 60, 50];
const MIN_SIDE: u32 = 64;
/// The library thumbnail fits this box (kindlegen's thumbnails are about 330 px wide).
const THUMB_BOX: (u32, u32) = (330, 470);

/// Bytes a page JPEG of `len` adds to the book besides itself: at most a standard copy, when it
/// is over [`LIMIT`].
pub fn copy_bound(len: u64) -> u64 {
    match len > LIMIT as u64 {
        true => LIMIT as u64,
        false => 0,
    }
}

/// `jpeg` itself when it fits a record, else a smaller copy (`Some`).
pub fn low_res(jpeg: &[u8]) -> Result<Option<Vec<u8>>> {
    if jpeg.len() <= LIMIT {
        return Ok(None);
    }
    let img = match is_gray(jpeg) {
        true => DynamicImage::ImageLuma8(decode(jpeg)?.to_luma8()),
        false => decode(jpeg)?,
    };
    let mut scaled = img.clone();
    loop {
        for quality in QUALITIES {
            let out = encode(&scaled, quality)?;
            if out.len() <= LIMIT {
                return Ok(Some(out));
            }
        }
        let (w, h) = (scaled.width() * 17 / 20, scaled.height() * 17 / 20);
        if w.min(h) < MIN_SIDE {
            return Err(Error::Other(
                "azw3: page can't be made to fit 128 KB".into(),
            ));
        }
        scaled = img.resize_exact(w, h, FilterType::Triangle);
    }
}

/// The cover shrunk to a library thumbnail, within the record limit like any standard image.
pub fn thumbnail(cover: &[u8]) -> Result<Vec<u8>> {
    let img = decode(cover)?;
    let thumb = encode(
        &img.resize(THUMB_BOX.0, THUMB_BOX.1, FilterType::Triangle),
        80,
    )?;
    Ok(low_res(&thumb)?.unwrap_or(thumb))
}

/// Whether the JPEG has a single (grey) component, read from its start-of-frame header: the
/// decoder hands grey JPEGs back as RGB, and a copy should keep the original's colour model.
fn is_gray(jpeg: &[u8]) -> bool {
    let mut at = 2;
    while let (Some(&0xFF), Some(&marker)) = (jpeg.get(at), jpeg.get(at + 1)) {
        let len = jpeg
            .get(at + 2..at + 4)
            .map_or(0, |b| usize::from(u16::from_be_bytes([b[0], b[1]])));
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            return jpeg.get(at + 9) == Some(&1);
        }
        at += 2 + len;
    }
    false
}

fn decode(jpeg: &[u8]) -> Result<DynamicImage> {
    Ok(image::load_from_memory_with_format(
        jpeg,
        ImageFormat::Jpeg,
    )?)
}

fn encode(img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    let img = match img {
        DynamicImage::ImageLuma8(_) | DynamicImage::ImageRgb8(_) => img.clone(),
        other => DynamicImage::ImageRgb8(other.to_rgb8()),
    };
    // `write_image` with the image's own colour type: `encode_image` widens grey to three
    // components.
    let color = img.color().into();
    JpegEncoder::new_with_quality(&mut out, quality).write_image(
        img.as_bytes(),
        img.width(),
        img.height(),
        color,
    )?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma};

    /// A noisy grey page, so it compresses badly.
    fn page(w: u32, h: u32) -> Vec<u8> {
        let img = GrayImage::from_fn(w, h, |x, y| Luma([((x * 7919 + y * 104729) % 251) as u8]));
        encode(&DynamicImage::ImageLuma8(img), 95).unwrap()
    }

    #[test]
    fn small_pages_are_kept_and_big_ones_shrunk_deterministically() {
        let small = page(64, 64);
        assert!(low_res(&small).unwrap().is_none());
        let big = page(1264, 1680);
        assert!(big.len() > LIMIT);
        let copy = low_res(&big).unwrap().unwrap();
        assert!(copy.len() <= LIMIT);
        assert_eq!(low_res(&big).unwrap().unwrap(), copy);
        assert!(is_gray(&big) && is_gray(&copy));
    }

    #[test]
    fn thumbnail_fits_the_box() {
        let t = decode(&thumbnail(&page(1264, 1680)).unwrap()).unwrap();
        assert!(t.width() <= 330 && t.height() <= 470);
    }

    #[test]
    fn a_noisy_colour_thumbnail_still_fits_a_record() {
        let img = image::RgbImage::from_fn(1264, 1680, |x, y| {
            let v = x.wrapping_mul(2_654_435_761) ^ y.wrapping_mul(40_503);
            image::Rgb([v as u8, (v >> 8) as u8, (v >> 16) as u8])
        });
        let cover = encode(&DynamicImage::ImageRgb8(img), 95).unwrap();
        assert!(thumbnail(&cover).unwrap().len() <= LIMIT);
    }
}
