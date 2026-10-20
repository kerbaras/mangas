//! A tall speech balloon (big shouted lettering, long tail) stays on one page.

use image::{Rgba, RgbaImage};
use mangas_core::smartcut::{Preset, StripPaginator};

const W: u32 = 720;
const BALLOON: (u32, u32) = (300, 1150);

fn fill(img: &mut RgbaImage, x: std::ops::Range<u32>, y: std::ops::Range<u32>, v: u8) {
    for yy in y {
        for xx in x.clone() {
            img.put_pixel(xx, yy, Rgba([v, v, v, 255]));
        }
    }
}

/// Black art, then an 850-row white balloon with a 3 px outline and one line of 40-row ring
/// glyphs near its top, then more black art: no gutter anywhere near the balloon.
fn strip() -> RgbaImage {
    let mut img = RgbaImage::from_pixel(W, 2400, Rgba([255, 255, 255, 255]));
    fill(&mut img, 20..W - 20, 0..BALLOON.0, 0);
    fill(&mut img, 20..W - 20, BALLOON.1..2400, 0);
    let (top, bottom) = BALLOON;
    fill(&mut img, 100..620, top..top + 3, 0);
    fill(&mut img, 100..620, bottom - 3..bottom, 0);
    fill(&mut img, 100..103, top..bottom, 0);
    fill(&mut img, 617..620, top..bottom, 0);
    for i in 0..8 {
        let x0 = 180 + i * 40;
        fill(&mut img, x0..x0 + 30, top + 60..top + 100, 0);
        fill(&mut img, x0 + 5..x0 + 25, top + 65..top + 95, 255);
    }
    img
}

#[test]
fn a_tall_balloon_is_never_cut() {
    let img = strip();
    let mut p = StripPaginator::new(1264, 1680, Preset::Balanced.options());
    let pages = p.feed(&img, true).expect("paginate");
    let spans: Vec<_> = pages.iter().map(|p| p.rows.clone()).collect();
    assert!(
        spans
            .iter()
            .any(|r| r.start <= BALLOON.0 as usize && r.end >= BALLOON.1 as usize),
        "balloon {BALLOON:?} split across {spans:?}"
    );
}
