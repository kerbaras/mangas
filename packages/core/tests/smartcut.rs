//! Smart cutting on synthetic strips: white background, black panels separated by gutters.

use image::{Rgba, RgbaImage};
use mangas_core::smartcut::{capacity, Options, Preset, StripPaginator};

const W: u32 = 709;
const DEV_W: u32 = 1264;
const DEV_H: u32 = 1680;

fn strip(panels: &[(u32, u32)]) -> RgbaImage {
    // panels: (panel height, gutter height after it)
    let h: u32 = panels.iter().map(|(p, g)| p + g).sum();
    let mut img = RgbaImage::from_pixel(W, h, Rgba([255, 255, 255, 255]));
    let mut y = 0;
    for &(p, g) in panels {
        for yy in y..y + p {
            for x in 20..W - 20 {
                img.put_pixel(x, yy, Rgba([0, 0, 0, 255]));
            }
        }
        y += p + g;
    }
    img
}

fn is_white_row(img: &RgbaImage, y: u32) -> bool {
    (0..img.width()).all(|x| img.get_pixel(x, y).0[..3] == [255, 255, 255])
}

fn paginate(img: &RgbaImage, opts: Options) -> Vec<mangas_core::smartcut::Page> {
    let mut p = StripPaginator::new(DEV_W, DEV_H, opts);
    p.feed(img, true).expect("paginate")
}

#[test]
fn cuts_land_on_gutters_and_cover_every_row() {
    let img = strip(&[(800, 80); 12]);
    let pages = paginate(&img, Preset::Balanced.options());
    assert!(pages.len() >= 10, "pages: {}", pages.len());
    assert_eq!(pages[0].rows.start, 0);
    assert_eq!(pages.last().unwrap().rows.end, img.height() as usize);
    for w in pages.windows(2) {
        assert!(
            w[1].rows.start <= w[0].rows.end,
            "gap between pages {:?} {:?}",
            w[0].rows,
            w[1].rows
        );
        let cut = w[0].rows.end as u32;
        assert!(
            is_white_row(&img, cut - 1) || is_white_row(&img, cut),
            "cut at {cut} is not on a gutter"
        );
    }
    for p in &pages {
        assert_eq!(p.image.width(), DEV_W);
        assert_eq!(p.image.height(), DEV_H);
    }
}

#[test]
fn shrink_stays_within_max_shrink() {
    let cap = capacity(W as usize, DEV_W, DEV_H);
    for preset in [Preset::Balanced, Preset::FewestCuts, Preset::NoShrink] {
        let opts = preset.options();
        let img = strip(&[(1000, 40), (1100, 40), (950, 40), (1200, 40), (700, 40)]);
        let pages = paginate(&img, opts);
        let limit = (cap as f64 * opts.max_shrink).floor() as usize;
        for p in &pages {
            assert!(
                p.rows.end - p.rows.start <= limit,
                "{preset:?}: span {:?} exceeds {limit}",
                p.rows
            );
        }
    }
}

#[test]
fn forced_cuts_overlap() {
    let cap = capacity(W as usize, DEV_W, DEV_H);
    let opts = Preset::NoShrink.options();
    let img = strip(&[(cap as u32 * 4, 0)]);
    let pages = paginate(&img, opts);
    assert!(pages.len() >= 4);
    let min_overlap = (cap as f64 * opts.overlap).floor() as usize;
    for w in pages.windows(2) {
        let overlap = w[0].rows.end.saturating_sub(w[1].rows.start);
        assert!(
            overlap >= min_overlap,
            "overlap {overlap} < {min_overlap} for {:?} -> {:?}",
            w[0].rows,
            w[1].rows
        );
    }
    assert_eq!(pages.last().unwrap().rows.end, img.height() as usize);
}

#[test]
fn same_width_strips_are_stitched_and_tail_carried() {
    let opts = Preset::Balanced.options();
    let a = strip(&[(800, 80), (800, 80), (500, 0)]);
    let b = strip(&[(300, 80), (800, 80)]);
    let mut p = StripPaginator::new(DEV_W, DEV_H, opts);
    let first = p.feed(&a, false).expect("feed a");
    assert!(p.has_pending(W));
    let second = p.feed(&b, true).expect("feed b");
    let all: Vec<_> = first.into_iter().chain(second).collect();
    let total = (a.height() + b.height()) as usize;
    assert_eq!(all.first().unwrap().rows.start, 0);
    assert_eq!(all.last().unwrap().rows.end, total);
    for w in all.windows(2) {
        assert!(w[1].rows.start <= w[0].rows.end);
    }
    // The 500-row panel split over the file boundary must stay on one page.
    assert!(
        all.iter()
            .any(|p| p.rows.start <= 1760 && p.rows.end >= 2560),
        "{:?}",
        all.iter().map(|p| p.rows).collect::<Vec<_>>()
    );
}

#[test]
fn short_image_is_not_a_long_strip() {
    assert!(!mangas_core::smartcut::is_long_strip(
        709, 1000, DEV_W, DEV_H
    ));
    assert!(mangas_core::smartcut::is_long_strip(
        709, 5000, DEV_W, DEV_H
    ));
}
