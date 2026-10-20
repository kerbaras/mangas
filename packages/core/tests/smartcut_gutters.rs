//! Long blank gutters on webtoon strips are skipped instead of becoming blank pages (#135).

use image::{Rgba, RgbaImage};
use mangas_core::smartcut::{capacity, Page, Preset, StripPaginator};

const W: u32 = 709;
const DEV_W: u32 = 1264;
const DEV_H: u32 = 1680;

/// White strip with black panels: (gutter above, panel height) pairs, then a trailing gutter.
fn strip(parts: &[(u32, u32)], tail: u32) -> RgbaImage {
    let h: u32 = parts.iter().map(|(g, p)| g + p).sum::<u32>() + tail;
    let mut img = RgbaImage::from_pixel(W, h, Rgba([255, 255, 255, 255]));
    let mut y = 0;
    for &(g, p) in parts {
        y += g;
        for yy in y..y + p {
            for x in 20..W - 20 {
                img.put_pixel(x, yy, Rgba([0, 0, 0, 255]));
            }
        }
        y += p;
    }
    img
}

fn paginate(img: &RgbaImage) -> Vec<Page> {
    let mut p = StripPaginator::new(DEV_W, DEV_H, Preset::Balanced.options());
    p.feed(img, true).expect("paginate")
}

fn background_share(page: &Page) -> f64 {
    let white = page.image.pixels().filter(|p| p.0[..3] == [255; 3]).count();
    white as f64 / (page.image.width() * page.image.height()) as f64
}

#[test]
fn long_gutters_never_become_pages() {
    let cap = capacity(W as usize, DEV_W, DEV_H) as u32;
    let img = strip(
        &[(cap * 3, 600), (cap * 2, 400), (40, 400), (cap * 4, 300)],
        cap * 3,
    );
    let pages = paginate(&img);
    let shares: Vec<f64> = pages.iter().map(background_share).collect();
    assert!(shares.iter().all(|s| *s < 0.95), "{shares:?}");
    assert_eq!(
        pages.len(),
        3,
        "{:?}",
        pages.iter().map(|p| p.rows).collect::<Vec<_>>()
    );
}

#[test]
fn every_panel_row_is_still_shown() {
    let cap = capacity(W as usize, DEV_W, DEV_H) as u32;
    let parts = [(cap * 2, cap / 2), (cap, cap * 2), (cap / 3, 200)];
    let img = strip(&parts, cap);
    let pages = paginate(&img);
    let mut y = 0;
    for &(g, p) in &parts {
        y += g;
        for row in (y..y + p).map(|r| r as usize) {
            assert!(
                pages
                    .iter()
                    .any(|pg| pg.rows.start <= row && row < pg.rows.end),
                "panel row {row} is on no page: {:?}",
                pages.iter().map(|p| p.rows).collect::<Vec<_>>()
            );
        }
        y += p;
    }
}

#[test]
fn page_starts_just_above_the_art_after_a_long_gutter() {
    let cap = capacity(W as usize, DEV_W, DEV_H) as u32;
    let img = strip(&[(100, 400), (cap * 2, 400)], 0);
    let pages = paginate(&img);
    let art = (100 + 400 + cap * 2) as usize;
    let last = pages.last().expect("pages");
    assert!(last.rows.start <= art && art - last.rows.start <= cap as usize / 20);
}

#[test]
fn a_solid_band_of_another_colour_is_art() {
    let cap = capacity(W as usize, DEV_W, DEV_H) as u32;
    let mut img = strip(&[(200, 400), (cap * 2, 400), (200, 400)], 200);
    let band = 200 + 400 + cap * 2..200 + 400 + cap * 2 + cap;
    for y in band.clone() {
        for x in 0..W {
            img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
        }
    }
    let pages = paginate(&img);
    let spans: Vec<_> = pages.iter().map(|p| p.rows).collect();
    for row in band.step_by(50).map(|r| r as usize) {
        assert!(
            spans.iter().any(|r| r.start <= row && row < r.end),
            "band row {row} is on no page: {spans:?}"
        );
    }
}

#[test]
fn a_blank_chunk_is_not_held_back() {
    let cap = capacity(W as usize, DEV_W, DEV_H) as u32;
    let mut p = StripPaginator::new(DEV_W, DEV_H, Preset::Balanced.options());
    let mut pages = p.feed(&strip(&[(100, cap * 3)], 100), false).expect("art");
    for _ in 0..3 {
        let blank = RgbaImage::from_pixel(W, cap * 2, Rgba([255, 255, 255, 255]));
        pages.extend(p.feed(&blank, false).expect("blank"));
        assert!(!p.has_pending_any(), "a blank chunk is carried forward");
    }
    pages.extend(p.flush().expect("flush"));
    assert!(!pages.is_empty());
    assert!(pages.iter().all(|pg| background_share(pg) < 0.95));
}

fn paint(img: &mut RgbaImage, rows: std::ops::Range<u32>, c: [u8; 3]) {
    for y in rows {
        for x in 0..W {
            img.put_pixel(x, y, Rgba([c[0], c[1], c[2], 255]));
        }
    }
}

#[test]
fn each_chapter_learns_its_own_background() {
    let cap = capacity(W as usize, DEV_W, DEV_H) as u32;
    let mut p = StripPaginator::new(DEV_W, DEV_H, Preset::Balanced.options());
    let mut dark = strip(&[(100, 400), (cap * 4, 400)], 100);
    paint(&mut dark, 500..500 + cap * 4, [0; 3]);
    p.feed(&dark, true).expect("dark chapter");
    p.new_chapter();
    let pages = p
        .feed(&strip(&[(100, 400), (cap * 2, 400)], 100), true)
        .expect("light chapter");
    let shares: Vec<f64> = pages.iter().map(background_share).collect();
    assert!(shares.iter().all(|s| *s < 0.95), "{shares:?}");
}

#[test]
fn carried_rows_are_counted_once() {
    let cap = capacity(W as usize, DEV_W, DEV_H) as u32;
    let mut p = StripPaginator::new(DEV_W, DEV_H, Preset::Balanced.options());
    let mut head = strip(&[(50, 200)], 0);
    head = image::imageops::crop_imm(&head, 0, 0, W, 250).to_image();
    let mut first = RgbaImage::new(W, cap / 3 + 250);
    paint(&mut first, 0..cap / 3, [0; 3]);
    image::imageops::replace(&mut first, &head, 0, (cap / 3) as i64);
    let mut pages = p.feed(&first, false).expect("first");
    for _ in 0..10 {
        pages.extend(p.feed(&strip(&[(5, 15)], 0), false).expect("chunk"));
    }
    assert!(p.has_pending_any(), "the short chunks stay carried");
    pages.extend(p.feed(&strip(&[(cap * 2, 400)], 100), true).expect("last"));
    let shares: Vec<f64> = pages.iter().map(background_share).collect();
    assert!(shares.iter().all(|s| *s < 0.95), "{shares:?}");
}
