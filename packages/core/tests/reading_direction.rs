//! Reading direction (#325): the OPF values Kindle accepts, the source's direction in the
//! library, and the series setting layered over it.

use std::io::Read;
use std::path::Path;

use image::{ExtendedColorType, RgbImage};
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId, ReadingDirection};
use mangas_core::export::epub::{self, Book, ChapterEntry, PageImage};
use mangas_core::export::kindle::process::encode_jpeg;
use mangas_core::export::{ExportPreset, PageSpool};
use mangas_core::library::{
    get_manga, mark_downloaded, open_in_memory, series_layer_for_dirs, set_export_preset,
    upsert_chapters, upsert_manga, Library,
};

fn opf(right_to_left: bool) -> String {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.epub");
    let img = RgbImage::from_pixel(60, 80, image::Rgb([200, 200, 200]));
    let jpeg = encode_jpeg(img.as_raw(), 60, 80, ExtendedColorType::Rgb8, 80).unwrap();
    let page = PageImage {
        jpeg,
        width: 60,
        height: 80,
    };
    let book = Book {
        id: "1".into(),
        title: "T".into(),
        authors: vec!["mangas".into()],
        language: "ja".into(),
        series: None,
        right_to_left,
        device_width: 60,
        device_height: 80,
        chapters: vec![ChapterEntry {
            title: "Ch 1".into(),
            first_page: 0,
        }],
        pages: PageSpool::from_pages(vec![page]).unwrap(),
        cover: None,
    };
    epub::write(&book, &out, false).unwrap();
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let mut opf = String::new();
    zip.by_name("OEBPS/content.opf")
        .unwrap()
        .read_to_string(&mut opf)
        .unwrap();
    opf
}

#[test]
fn the_opf_uses_the_writing_modes_kindle_accepts() {
    let rtl = opf(true);
    assert!(rtl.contains(r#"<meta name="primary-writing-mode" content="horizontal-rl"/>"#));
    assert!(rtl.contains(r#"<spine page-progression-direction="rtl">"#));
    let ltr = opf(false);
    assert!(ltr.contains(r#"<meta name="primary-writing-mode" content="horizontal-lr"/>"#));
    assert!(ltr.contains(r#"<spine page-progression-direction="ltr">"#));
    for opf in [rtl, ltr] {
        assert!(!opf.contains("horizontal-ltr") && !opf.contains("horizontal-rtl"));
    }
}

fn follow(lib: &Library, direction: Option<ReadingDirection>) -> Manga {
    let manga = Manga {
        id: MangaId("m".into()),
        source: "mangadex".into(),
        title: "M".into(),
        direction,
        ..Default::default()
    };
    upsert_manga(lib, &manga).unwrap();
    let chapter = Chapter {
        id: ChapterId("c1".into()),
        manga_id: manga.id.clone(),
        language: "en".into(),
        number: "1".into(),
        ..Default::default()
    };
    upsert_chapters(lib, "mangadex", &manga.id, "en", &[chapter]).unwrap();
    mark_downloaded(
        lib,
        "mangadex",
        &ChapterId("c1".into()),
        Path::new("/lib/m/1"),
    )
    .unwrap();
    manga
}

#[test]
fn the_library_keeps_the_sources_direction_until_it_says_another() {
    let lib = open_in_memory().unwrap();
    let manga = follow(&lib, Some(ReadingDirection::Rtl));
    let stored = |lib: &Library| get_manga(lib, "mangadex", &manga.id).unwrap().unwrap();
    assert_eq!(stored(&lib).direction, Some(ReadingDirection::Rtl));
    // A refresh from a source that doesn't say keeps what an earlier one said.
    upsert_manga(
        &lib,
        &Manga {
            direction: None,
            ..manga.clone()
        },
    )
    .unwrap();
    assert_eq!(stored(&lib).direction, Some(ReadingDirection::Rtl));
    let ltr = Some(ReadingDirection::Ltr);
    upsert_manga(
        &lib,
        &Manga {
            direction: ltr,
            ..manga.clone()
        },
    )
    .unwrap();
    assert_eq!(stored(&lib).direction, ltr);
}

#[test]
fn the_series_setting_beats_the_sources_direction() {
    let lib = open_in_memory().unwrap();
    follow(&lib, Some(ReadingDirection::Rtl));
    let dirs = [Path::new("/lib/m/1")];
    let layer = |lib: &Library| {
        series_layer_for_dirs(lib, &dirs)
            .unwrap()
            .unwrap()
            .direction
    };
    assert_eq!(layer(&lib), Some(ReadingDirection::Rtl));
    let setting = ExportPreset {
        direction: Some(ReadingDirection::Ltr),
        ..Default::default()
    };
    set_export_preset(&lib, "mangadex", "m", &setting).unwrap();
    assert_eq!(layer(&lib), Some(ReadingDirection::Ltr));
    assert_eq!(
        series_layer_for_dirs(&lib, &[Path::new("/x")]).unwrap(),
        None
    );
}

#[test]
fn a_series_without_a_direction_reads_left_to_right() {
    let lib = open_in_memory().unwrap();
    follow(&lib, None);
    let layer = series_layer_for_dirs(&lib, &[Path::new("/lib/m/1")]).unwrap();
    assert_eq!(
        layer.and_then(|p| p.direction).unwrap_or_default(),
        ReadingDirection::Ltr
    );
}
