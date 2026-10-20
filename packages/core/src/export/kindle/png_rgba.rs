//! 8-bit PNG pages decoded row by row straight into RGBA: the same pixels as `image`'s
//! `decode().into_rgba8()` without its whole RGB copy of the page next to the RGBA one (a
//! 2722x3870 page is 32 MB of RGB plus 42 MB of RGBA).

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use image::RgbaImage;
use png::{BitDepth, ColorType, Transformations};

/// `image`'s default allocation limit: a bigger page goes to the general decoder, which refuses it.
const MAX_BYTES: u64 = 512 * 1024 * 1024;

/// The page, or `None` when `path` needs the general decoder (not a PNG, interlaced, 16-bit,
/// too big, or unreadable, so that decoder reports the error).
pub(super) fn decode(path: &Path) -> Option<RgbaImage> {
    let mut decoder = png::Decoder::new(BufReader::new(File::open(path).ok()?));
    decoder.set_transformations(Transformations::EXPAND);
    let mut reader = decoder.read_info().ok()?;
    let (color, depth) = reader.output_color_type();
    if reader.info().interlaced || depth != BitDepth::Eight {
        return None;
    }
    let (w, h) = (reader.info().width, reader.info().height);
    if !fits(w, h) {
        return None;
    }
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    while let Some(row) = reader.next_row().ok()? {
        to_rgba(color, row.data(), &mut out)?;
    }
    RgbaImage::from_raw(w, h, out)
}

fn fits(w: u32, h: u32) -> bool {
    (u64::from(w) * u64::from(h))
        .checked_mul(4)
        .is_some_and(|bytes| bytes <= MAX_BYTES)
}

fn to_rgba(color: ColorType, row: &[u8], out: &mut Vec<u8>) -> Option<()> {
    match color {
        ColorType::Rgba => out.extend_from_slice(row),
        ColorType::Rgb => row
            .as_chunks::<3>()
            .0
            .iter()
            .for_each(|p| out.extend_from_slice(&[p[0], p[1], p[2], 255])),
        ColorType::Grayscale => row
            .iter()
            .for_each(|&l| out.extend_from_slice(&[l, l, l, 255])),
        ColorType::GrayscaleAlpha => row
            .as_chunks::<2>()
            .0
            .iter()
            .for_each(|p| out.extend_from_slice(&[p[0], p[0], p[0], p[1]])),
        ColorType::Indexed => return None,
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, GrayAlphaImage, GrayImage, RgbImage};

    fn same_as_image(img: DynamicImage) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.png");
        img.save(&path).unwrap();
        let general = image::open(&path).unwrap().into_rgba8();
        assert_eq!(decode(&path).unwrap(), general);
    }

    #[test]
    fn gives_the_general_decoders_pixels() {
        let px = |x: u32, y: u32| (x * 31 + y * 7) as u8;
        same_as_image(RgbImage::from_fn(9, 5, |x, y| image::Rgb([px(x, y), 3, px(y, x)])).into());
        same_as_image(GrayImage::from_fn(9, 5, |x, y| image::Luma([px(x, y)])).into());
        same_as_image(GrayAlphaImage::from_fn(9, 5, |x, y| image::LumaA([px(x, y), 9])).into());
        same_as_image(RgbaImage::from_fn(9, 5, |x, y| image::Rgba([px(x, y), 1, 2, 3])).into());
    }

    #[test]
    fn pages_past_the_allocation_limit_go_to_the_general_decoder() {
        assert!(fits(2722, 3870));
        assert!(!fits(100_000, 100_000));
        assert!(!fits(u32::MAX, u32::MAX));
    }

    #[test]
    fn a_huge_header_is_refused_before_any_pixels_are_allocated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("huge.png");
        let mut bytes = Vec::new();
        let mut enc = png::Encoder::new(&mut bytes, 100_000, 100_000);
        enc.set_color(ColorType::Rgba);
        drop(enc.write_header().unwrap());
        std::fs::write(&path, &bytes).unwrap();
        assert!(decode(&path).is_none());
    }

    #[test]
    fn leaves_other_files_to_the_general_decoder() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.jpg");
        RgbImage::new(4, 4).save(&path).unwrap();
        assert!(decode(&path).is_none());
        assert!(decode(&dir.path().join("missing.png")).is_none());
    }
}
