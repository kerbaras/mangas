//! QA tool for the AZW3 writer until #329 wires it into the CLI: turns an EPUB written by
//! `mangas export --format epub` (or `mangas kindle`) into a fixed-layout AZW3 with the same
//! page JPEGs, title, authors, language, direction and chapter TOC.
//!
//! cargo run --release -p mangas-core --example azw3_from_epub -- BOOK.epub out.azw3 [PROFILE]

use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

use mangas_core::export::azw3::Azw3Exporter;
use mangas_core::export::epub::{ChapterEntry, PageImage};
use mangas_core::export::{profile, Exporter, Labels, Meta, PageSpool};
use roxmltree::{Document, Node};

type Res<T> = Result<T, Box<dyn std::error::Error>>;
type Zip = zip::ZipArchive<File>;

fn main() -> Res<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [epub, out, rest @ ..] = args.as_slice() else {
        return Err("usage: azw3_from_epub BOOK.epub out.azw3 [PROFILE]".into());
    };
    let profile_id = rest.first().map_or("kindle-paperwhite12", String::as_str);
    let profile = profile(profile_id).ok_or("unknown profile")?;
    let mut zip = zip::ZipArchive::new(File::open(epub)?)?;
    let opf_path = rootfile(&mut zip)?;
    let opf = text(&mut zip, &opf_path)?;
    let doc = parse(&opf)?;
    let base = opf_path.rsplit_once('/').map_or("", |(dir, _)| dir);
    let spine = spine_files(&doc, base)?;
    let mut cover = None;
    let mut pages = Vec::new();
    let mut page_files = Vec::new();
    for xhtml in &spine {
        let image = image_of(&mut zip, xhtml)?;
        let jpeg = bytes(&mut zip, &image)?;
        let (width, height) = image::ImageReader::new(Cursor::new(&jpeg))
            .with_guessed_format()?
            .into_dimensions()?;
        let page = PageImage {
            jpeg,
            width,
            height,
        };
        match xhtml.ends_with("cover.xhtml") && cover.is_none() && pages.is_empty() {
            true => cover = Some(page),
            false => {
                pages.push(page);
                page_files.push(xhtml.clone());
            }
        }
    }
    let chapters = toc(&mut zip, &doc, base, &page_files)?;
    let all = |name: &str| -> Vec<String> {
        (doc.descendants().filter(|n| n.tag_name().name() == name))
            .filter_map(|n| n.text().map(str::to_string))
            .collect()
    };
    let first = |name: &str| all(name).into_iter().next().unwrap_or_default();
    let rtl = (doc.descendants().find(|n| n.tag_name().name() == "spine"))
        .and_then(|n| n.attribute("page-progression-direction"))
        == Some("rtl");
    let meta = Meta {
        id: first("identifier"),
        title: first("title"),
        authors: all("creator"),
        language: first("language"),
        series: None,
        right_to_left: rtl,
        chapters,
        labels: Labels::default(),
        title_explicit: true,
        cover,
    };
    let spool_len = pages.len();
    let spool = PageSpool::from_pages(pages)?;
    let written = Azw3Exporter.export(spool, &meta, &profile, Path::new(out), true)?;
    println!(
        "{}: {} pages, rtl={rtl}",
        written.display(),
        meta.cover.iter().count() + spool_len
    );
    Ok(())
}

fn rootfile(zip: &mut Zip) -> Res<String> {
    let container = text(zip, "META-INF/container.xml")?;
    let doc = parse(&container)?;
    let root = doc
        .descendants()
        .find(|n| n.tag_name().name() == "rootfile");
    Ok(root
        .and_then(|n| n.attribute("full-path"))
        .ok_or("no rootfile")?
        .to_string())
}

/// The spine's XHTML files, as zip paths.
fn spine_files(doc: &Document, base: &str) -> Res<Vec<String>> {
    let items: Vec<Node> = doc
        .descendants()
        .filter(|n| n.tag_name().name() == "item")
        .collect();
    let href = |id: &str| {
        items
            .iter()
            .find(|i| i.attribute("id") == Some(id))
            .and_then(|i| i.attribute("href"))
    };
    (doc.descendants()
        .filter(|n| n.tag_name().name() == "itemref"))
    .map(|r| {
        let id = r.attribute("idref").ok_or("itemref without idref")?;
        Ok(join(base, href(id).ok_or("spine item not in manifest")?))
    })
    .collect()
}

fn image_of(zip: &mut Zip, xhtml: &str) -> Res<String> {
    let page = text(zip, xhtml)?;
    let doc = parse(&page)?;
    let img = doc.descendants().find(|n| n.tag_name().name() == "img");
    let src = img
        .and_then(|n| n.attribute("src"))
        .ok_or("page without img")?;
    let dir = xhtml.rsplit_once('/').map_or("", |(d, _)| d);
    Ok(join(dir, src))
}

/// The nav document's top-level entries, as chapters starting at their page.
fn toc(zip: &mut Zip, doc: &Document, base: &str, pages: &[String]) -> Res<Vec<ChapterEntry>> {
    let nav = doc
        .descendants()
        .find(|n| n.attribute("properties") == Some("nav"));
    let Some(href) = nav.and_then(|n| n.attribute("href")) else {
        return Ok(Vec::new());
    };
    let nav_path = join(base, href);
    let nav = text(zip, &nav_path)?;
    let nav_doc = parse(&nav)?;
    let dir = nav_path.rsplit_once('/').map_or("", |(d, _)| d);
    let toc = nav_doc
        .descendants()
        .find(|n| n.attribute(("http://www.idpf.org/2007/ops", "type")) == Some("toc"));
    let links = toc
        .into_iter()
        .flat_map(|t| t.descendants())
        .filter(|n| n.tag_name().name() == "a");
    Ok(links
        .filter_map(|a| {
            let target = join(dir, a.attribute("href")?.split('#').next()?);
            let first_page = pages.iter().position(|p| *p == target)?;
            let title = (a.descendants().filter(|n| n.is_text()))
                .filter_map(|n| n.text())
                .collect::<String>();
            Some(ChapterEntry { title, first_page })
        })
        .collect())
}

/// `href` relative to the zip folder `dir`, with `..` resolved.
fn join(dir: &str, href: &str) -> String {
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for part in href.split('/') {
        match part {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            p => parts.push(p),
        }
    }
    parts.join("/")
}

fn parse(xml: &str) -> Res<Document<'_>> {
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    Ok(Document::parse_with_options(xml, options)?)
}

fn bytes(zip: &mut Zip, name: &str) -> Res<Vec<u8>> {
    let mut out = Vec::new();
    zip.by_name(name)?.read_to_end(&mut out)?;
    Ok(out)
}

fn text(zip: &mut Zip, name: &str) -> Res<String> {
    Ok(String::from_utf8(bytes(zip, name)?)?)
}
