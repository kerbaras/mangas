//! Writers stream pages from the spool: peak heap stays near one page, not the whole book.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use image::{ExtendedColorType, RgbImage};
use mangas_core::export::epub::{ChapterEntry, PageImage};
use mangas_core::export::kindle::process::encode_jpeg;
use mangas_core::export::{profile, Format, Meta, PageSpool};

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
            PEAK.fetch_max(live, Ordering::SeqCst);
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

const PAGES: usize = 40;

/// A ~1 MB JPEG of noise, so neither zip nor JPEG shrinks it much.
fn noisy_page() -> PageImage {
    let (w, h) = (1264, 1680);
    let mut seed = 0x2545_f491_u32;
    let img = RgbImage::from_fn(w, h, |_, _| {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        image::Rgb([seed as u8, (seed >> 8) as u8, (seed >> 16) as u8])
    });
    let jpeg = encode_jpeg(img.as_raw(), w, h, ExtendedColorType::Rgb8, 90).unwrap();
    PageImage {
        jpeg,
        width: w,
        height: h,
    }
}

fn meta() -> Meta {
    Meta {
        id: "memory".into(),
        title: "Memory".into(),
        authors: vec!["mangas".into()],
        language: "en".into(),
        series: None,
        right_to_left: false,
        chapters: vec![ChapterEntry {
            title: "Ch 1".into(),
            first_page: 0,
        }],
        labels: Default::default(),
        title_explicit: false,
        cover: None,
    }
}

#[test]
fn every_format_writes_with_about_one_page_in_memory() {
    let dir = tempfile::tempdir().unwrap();
    let page = noisy_page();
    let page_bytes = page.jpeg.len();
    assert!(page_bytes > 500_000, "noise page is {page_bytes} bytes");
    let dev = profile("kindle-paperwhite12").unwrap();
    for format in [Format::Epub, Format::Cbz, Format::Pdf] {
        let mut spool = PageSpool::new().unwrap();
        for _ in 0..PAGES {
            spool.push(&page).unwrap();
        }
        let base = LIVE.load(Ordering::SeqCst);
        PEAK.store(base, Ordering::SeqCst);
        let out = dir.path().join(format!("book.{}", format.id()));
        format
            .exporter()
            .export(spool, &meta(), &dev, &out, false)
            .unwrap();
        let grew = PEAK.load(Ordering::SeqCst) - base;
        let book = page_bytes * PAGES;
        assert!(
            grew < page_bytes * 4,
            "{format:?} peaked {grew} bytes over the spool for a {book}-byte book"
        );
        assert!(std::fs::metadata(&out).unwrap().len() as usize > book * 9 / 10);
    }
}
