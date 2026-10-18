//! A planned volume exported end to end: unzip the EPUB and parse its OPF and nav.

use std::io::Read;
use std::path::Path;

use image::{Rgba, RgbaImage};
use mangas_core::domain::{Chapter, ChapterId, Manga};
use mangas_core::export::kindle::{device, export};
use mangas_core::export::volume::{plan, request, Grouping};
use mangas_core::smartcut::Preset;

const OPF_NS: &str = "http://www.idpf.org/2007/opf";
const OPS_NS: &str = "http://www.idpf.org/2007/ops";

fn png(path: &Path, w: u32, h: u32) {
    RgbaImage::from_pixel(w, h, Rgba([240, 240, 240, 255]))
        .save(path)
        .unwrap();
}

fn chapter(root: &Path, number: &str, pages: usize) -> Chapter {
    let dir = root.join(format!("ch{number}"));
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..pages {
        png(&dir.join(format!("{i:03}.png")), 800, 1200);
    }
    Chapter {
        id: ChapterId(number.into()),
        number: number.into(),
        title: (number == "1").then(|| "Fish & chips".into()),
        volume: Some("2".into()),
        language: "ja".into(),
        local_path: Some(dir.to_string_lossy().into_owned()),
        ..Default::default()
    }
}

fn entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> String {
    let mut s = String::new();
    zip.by_name(name).unwrap().read_to_string(&mut s).unwrap();
    s
}

fn export_volume(root: &Path, cover: Option<std::path::PathBuf>) -> zip::ZipArchive<std::fs::File> {
    let chapters = vec![chapter(root, "1", 2), chapter(root, "2", 1)];
    let manga = Manga {
        title: "Frieren".into(),
        authors: vec!["Kanehito Yamada".into()],
        artists: vec!["Tsukasa Abe".into()],
        ..Default::default()
    };
    let plans = plan(&manga, &chapters, Grouping::ByVolume, None, None);
    let dev = device("kindle-paperwhite12").unwrap();
    let out = root.join("out");
    let req = request(
        &manga,
        &plans[0],
        dev,
        Preset::Balanced.options(),
        &out,
        cover,
    );
    let stats = export(&req, None, &mut |_| {}).unwrap();
    assert_eq!(stats.output, out.join("Frieren v2.epub"));
    zip::ZipArchive::new(std::fs::File::open(&stats.output).unwrap()).unwrap()
}

#[test]
fn volume_epub_has_series_metadata_a_cover_page_and_a_chapter_toc() {
    let root = tempfile::tempdir().unwrap();
    let cover = root.path().join("cover.png");
    png(&cover, 500, 750);
    let mut zip = export_volume(root.path(), Some(cover));

    let opf = entry(&mut zip, "OEBPS/content.opf");
    let doc = roxmltree::Document::parse(&opf).unwrap();
    let text = |tag: &str| -> Vec<String> {
        doc.descendants()
            .filter(|n| n.tag_name().name() == tag)
            .filter_map(|n| n.text().map(str::to_string))
            .collect()
    };
    assert_eq!(text("title"), ["Frieren v2"]);
    assert_eq!(text("creator"), ["Kanehito Yamada", "Tsukasa Abe"]);
    assert_eq!(text("language"), ["ja"]);
    let meta = |attr: &str, value: &str| {
        doc.descendants()
            .find(|n| n.tag_name().name() == "meta" && n.attribute(attr) == Some(value))
    };
    assert_eq!(
        meta("property", "belongs-to-collection").and_then(|n| n.text()),
        Some("Frieren")
    );
    assert_eq!(
        meta("property", "group-position").and_then(|n| n.text()),
        Some("2")
    );
    assert_eq!(
        meta("name", "calibre:series_index").and_then(|n| n.attribute("content")),
        Some("2")
    );
    assert_eq!(
        meta("name", "cover").and_then(|n| n.attribute("content")),
        Some("cover-img")
    );
    let spine: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name((OPF_NS, "itemref")))
        .filter_map(|n| n.attribute("idref"))
        .collect();
    assert_eq!(
        spine.first(),
        Some(&"cover"),
        "the cover page opens the book"
    );
    assert_eq!(spine.len(), 4, "cover + three pages");
    let cover_items: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute("properties") == Some("cover-image"))
        .filter_map(|n| n.attribute("href"))
        .collect();
    assert_eq!(cover_items, ["images/cover.jpg"]);
    assert!(zip.by_name("OEBPS/text/cover.xhtml").is_ok());

    let nav = entry(&mut zip, "OEBPS/nav.xhtml");
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let doc = roxmltree::Document::parse_with_options(&nav, options).unwrap();
    let navs: Vec<_> = doc
        .descendants()
        .filter(|n| n.tag_name().name() == "nav")
        .collect();
    let toc = navs
        .iter()
        .find(|n| n.attribute((OPS_NS, "type")) == Some("toc"))
        .unwrap();
    let links: Vec<_> = toc
        .descendants()
        .filter(|n| n.tag_name().name() == "a")
        .map(|a| {
            (
                a.text().unwrap_or_default(),
                a.attribute("href").unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        links,
        [
            ("Chapter 1: Fish & chips", "text/page-00000.xhtml"),
            ("Chapter 2", "text/page-00002.xhtml"),
        ]
    );
    let landmarks = navs
        .iter()
        .find(|n| n.attribute((OPS_NS, "type")) == Some("landmarks"))
        .unwrap();
    let cover_link = landmarks
        .descendants()
        .find(|n| n.attribute((OPS_NS, "type")) == Some("cover"))
        .and_then(|n| n.attribute("href"));
    assert_eq!(cover_link, Some("text/cover.xhtml"));
}

#[test]
fn without_a_series_cover_the_first_page_is_the_cover_and_not_repeated() {
    let root = tempfile::tempdir().unwrap();
    let mut zip = export_volume(root.path(), Some(root.path().join("missing.jpg")));
    let opf = entry(&mut zip, "OEBPS/content.opf");
    let doc = roxmltree::Document::parse(&opf).unwrap();
    let cover_items: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute("properties") == Some("cover-image"))
        .filter_map(|n| n.attribute("href"))
        .collect();
    assert_eq!(cover_items, ["images/page-00000.jpg"]);
    assert_eq!(opf.matches("<itemref ").count(), 3);
    assert!(zip.by_name("OEBPS/text/cover.xhtml").is_err());
}

#[test]
fn exporting_the_same_chapters_again_keeps_the_book_identifier() {
    let identifier = |root: &Path| {
        let mut zip = export_volume(root, None);
        let opf = entry(&mut zip, "OEBPS/content.opf");
        let doc = roxmltree::Document::parse(&opf).unwrap();
        doc.descendants()
            .find(|n| n.tag_name().name() == "identifier")
            .and_then(|n| n.text())
            .unwrap()
            .to_string()
    };
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let first = identifier(a.path());
    assert_eq!(first, identifier(b.path()));
    let uuid = first.strip_prefix("urn:uuid:").unwrap();
    assert_eq!(uuid.chars().nth(14), Some('5'), "a UUIDv5: {uuid}");
}
