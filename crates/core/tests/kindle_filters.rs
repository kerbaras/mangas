//! E-ink filters and decode sanity.

use std::io::Cursor;

use image::{ImageFormat, Rgba, RgbaImage};
use mangas_core::export::kindle::filters::{
    apply_lut, grayscale_rgba, is_identity, sharpen_gray, tone_lut,
};
use mangas_core::export::kindle::process::{fit_dimensions, process_page, ImageSettings};
use mangas_core::export::{self};

#[test]
fn webp_black_decodes_to_zero() {
    let img = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 255]));
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::WebP).unwrap();
    let decoded = image::load_from_memory(buf.get_ref()).unwrap().to_rgba8();
    assert!(decoded.pixels().all(|p| p.0[..3] == [0, 0, 0]));
    let gray = grayscale_rgba(decoded.as_raw());
    assert!(gray.iter().all(|&v| v == 0));
}

#[test]
fn grayscale_is_bt601() {
    assert_eq!(grayscale_rgba(&[255, 0, 0, 255]), vec![76]);
    assert_eq!(grayscale_rgba(&[0, 255, 0, 255]), vec![150]);
    assert_eq!(grayscale_rgba(&[0, 0, 255, 255]), vec![29]);
    assert_eq!(grayscale_rgba(&[255, 255, 255, 255]), vec![255]);
}

#[test]
fn tone_lut_combines_contrast_and_gamma() {
    assert!(is_identity(&tone_lut(1.0, 1.0)));
    let lut = tone_lut(1.1, 0.9);
    assert_eq!(lut[0], 0);
    assert_eq!(lut[255], 255);
    assert!(lut[128] < 128, "gamma 0.9 darkens midtones: {}", lut[128]);
    assert!(lut[40] < 40 && lut[220] > 220, "contrast spreads the ends");
    assert_eq!(
        apply_lut(&[0, 128, 255], &lut),
        vec![lut[0], lut[128], lut[255]]
    );
}

#[test]
fn sharpen_boosts_edges_and_keeps_flat_areas() {
    let flat = vec![100u8; 25];
    assert_eq!(sharpen_gray(&flat, 5, 5), flat);
    let mut edge = vec![255u8; 25];
    for y in 0..5 {
        edge[y * 5 + 2] = 0;
        edge[y * 5 + 3] = 0;
        edge[y * 5 + 4] = 0;
    }
    let out = sharpen_gray(&edge, 5, 5);
    assert_eq!(out[2 * 5 + 1], 255);
    assert_eq!(out[2 * 5 + 2], 0);
    assert_eq!(out[0], 255, "border copied");
}

#[test]
fn process_page_fits_and_encodes_gray_jpeg() {
    let dev = export::kindle::device("kindle-paperwhite12").unwrap();
    let s = ImageSettings::for_device(&dev);
    assert_eq!(s.quality, 90);
    assert!(s.grayscale && s.sharpen);
    assert_eq!(fit_dimensions(2528, 3360, 1264, 1680), (1264, 1680));
    assert_eq!(fit_dimensions(600, 800, 1264, 1680), (600, 800));
    let img = RgbaImage::from_pixel(2000, 1000, Rgba([255, 0, 0, 255]));
    let page = process_page(&img, &s).unwrap();
    assert_eq!((page.width, page.height), (1264, 632));
    let decoded = image::load_from_memory(&page.jpeg).unwrap();
    assert_eq!(decoded.color(), image::ColorType::L8);
}
