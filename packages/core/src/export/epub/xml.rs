//! XML documents of a fixed-layout EPUB3 comic. Pure string builders.

use super::{Book, PageEntry, PageImage, Series, COVER_IMAGE, COVER_XHTML};

pub(super) fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub const CONTAINER_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"#;

/// One `dc:creator` per author, each refined with the MARC `aut` role.
fn creators(authors: &[String]) -> String {
    authors
        .iter()
        .enumerate()
        .map(|(i, a)| {
            format!(
                "    <dc:creator id=\"creator{i}\">{}</dc:creator>\n    <meta refines=\"#creator{i}\" property=\"role\" scheme=\"marc:relators\">aut</meta>\n",
                escape(a)
            )
        })
        .collect()
}

/// EPUB3 collection metadata plus the Calibre metas that Calibre and Kindle tools read.
/// The position is only written when it is a number, as both formats expect.
fn series_meta(series: Option<&Series>) -> String {
    let Some(s) = series else {
        return String::new();
    };
    let name = escape(&s.name);
    let position = s
        .position
        .as_deref()
        .filter(|p| p.trim().parse::<f64>().is_ok_and(f64::is_finite))
        .map(str::trim);
    let mut out = format!(
        "    <meta property=\"belongs-to-collection\" id=\"series\">{name}</meta>\n    <meta refines=\"#series\" property=\"collection-type\">series</meta>\n"
    );
    if let Some(p) = position {
        out.push_str(&format!(
            "    <meta refines=\"#series\" property=\"group-position\">{p}</meta>\n"
        ));
    }
    out.push_str(&format!(
        "    <meta name=\"calibre:series\" content=\"{name}\"/>\n"
    ));
    if let Some(p) = position {
        out.push_str(&format!(
            "    <meta name=\"calibre:series_index\" content=\"{p}\"/>\n"
        ));
    }
    out
}

fn manifest_and_spine(book: &Book, pages: &[PageEntry]) -> (String, String) {
    let mut manifest = String::new();
    let mut spine = String::new();
    if book.cover.is_some() {
        manifest.push_str(&format!(
            "    <item id=\"cover-img\" href=\"images/{COVER_IMAGE}\" media-type=\"image/jpeg\" properties=\"cover-image\"/>\n    <item id=\"cover\" href=\"text/{COVER_XHTML}\" media-type=\"application/xhtml+xml\"/>\n"
        ));
        spine.push_str("    <itemref idref=\"cover\"/>\n");
    }
    for p in pages {
        let cover = if book.cover.is_none() && p.index == 0 {
            " properties=\"cover-image\""
        } else {
            ""
        };
        manifest.push_str(&format!(
            "    <item id=\"img{0}\" href=\"images/{1}\" media-type=\"image/jpeg\"{cover}/>\n    <item id=\"pg{0}\" href=\"text/{2}\" media-type=\"application/xhtml+xml\"/>\n",
            p.index, p.image_name, p.xhtml_name
        ));
        spine.push_str(&format!("    <itemref idref=\"pg{}\"/>\n", p.index));
    }
    (manifest, spine)
}

pub fn content_opf(book: &Book, pages: &[PageEntry], modified: &str) -> String {
    let (w, h) = (book.device_width, book.device_height);
    let (manifest, spine) = manifest_and_spine(book, pages);
    let cover_id = if book.cover.is_some() {
        "cover-img"
    } else {
        "img0"
    };
    let direction = if book.right_to_left { "rtl" } else { "ltr" };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid" prefix="rendition: http://www.idpf.org/vocab/rendition/#">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="uid">urn:uuid:{id}</dc:identifier>
    <dc:title>{title}</dc:title>
{creators}    <dc:language>{lang}</dc:language>
{series}    <meta property="dcterms:modified">{modified}</meta>
    <meta property="rendition:layout">pre-paginated</meta>
    <meta property="rendition:orientation">portrait</meta>
    <meta property="rendition:spread">none</meta>
    <meta name="cover" content="{cover_id}"/>
    <meta name="fixed-layout" content="true"/>
    <meta name="original-resolution" content="{w}x{h}"/>
    <meta name="book-type" content="comic"/>
    <meta name="primary-writing-mode" content="horizontal-{direction}"/>
    <meta name="zero-gutter" content="true"/>
    <meta name="zero-margin" content="true"/>
    <meta name="orientation-lock" content="portrait"/>
    <meta name="RegionMagnification" content="false"/>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
{manifest}  </manifest>
  <spine page-progression-direction="{direction}">
{spine}  </spine>
</package>
"#,
        id = book.id,
        title = escape(&book.title),
        creators = creators(&book.authors),
        lang = escape(&book.language),
        series = series_meta(book.series.as_ref()),
    )
}

/// A full-bleed image page; `label` names it in the head title and the image's alt text.
fn image_xhtml(title: &str, label: &str, image: &str, w: u32, h: u32) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head>
  <title>{title} - {label}</title>
  <meta name="viewport" content="width={w}, height={h}"/>
  <style>html, body {{ margin: 0; padding: 0; width: {w}px; height: {h}px; }} img {{ display: block; width: {w}px; height: {h}px; }}</style>
</head>
<body>
  <div><img src="../images/{image}" alt="{label}"/></div>
</body>
</html>
"#,
        title = escape(title),
    )
}

pub fn page_xhtml(title: &str, p: &PageEntry) -> String {
    let label = format!("Page {}", p.index + 1);
    image_xhtml(title, &label, &p.image_name, p.width, p.height)
}

pub fn cover_xhtml(title: &str, cover: &PageImage) -> String {
    image_xhtml(title, "Cover", COVER_IMAGE, cover.width, cover.height)
}
