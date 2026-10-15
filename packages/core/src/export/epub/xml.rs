//! XML documents of a fixed-layout EPUB3 comic. Pure string builders.

use super::{Book, PageEntry};

fn escape(s: &str) -> String {
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

pub fn content_opf(book: &Book, pages: &[PageEntry], modified: &str) -> String {
    let (w, h) = (book.device_width, book.device_height);
    let mut manifest = String::new();
    let mut spine = String::new();
    for p in pages {
        manifest.push_str(&format!(
            "    <item id=\"img{0}\" href=\"images/{1}\" media-type=\"image/jpeg\"/>\n    <item id=\"pg{0}\" href=\"text/{2}\" media-type=\"application/xhtml+xml\"/>\n",
            p.index, p.image_name, p.xhtml_name
        ));
        spine.push_str(&format!("    <itemref idref=\"pg{}\"/>\n", p.index));
    }
    let direction = if book.right_to_left { "rtl" } else { "ltr" };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid" prefix="rendition: http://www.idpf.org/vocab/rendition/#">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="uid">urn:uuid:{id}</dc:identifier>
    <dc:title>{title}</dc:title>
    <dc:creator>{author}</dc:creator>
    <dc:language>{lang}</dc:language>
    <meta property="dcterms:modified">{modified}</meta>
    <meta property="rendition:layout">pre-paginated</meta>
    <meta property="rendition:orientation">portrait</meta>
    <meta property="rendition:spread">none</meta>
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
        author = escape(&book.author),
        lang = escape(&book.language),
    )
}

pub fn page_xhtml(title: &str, p: &PageEntry) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head>
  <title>{title} - {n}</title>
  <meta name="viewport" content="width={w}, height={h}"/>
  <style>html, body {{ margin: 0; padding: 0; width: {w}px; height: {h}px; }} img {{ display: block; width: {w}px; height: {h}px; }}</style>
</head>
<body>
  <div><img src="../images/{img}" alt="Page {n}"/></div>
</body>
</html>
"#,
        title = escape(title),
        n = p.index + 1,
        w = p.width,
        h = p.height,
        img = p.image_name,
    )
}

pub fn nav_xhtml(book: &Book, pages: &[PageEntry]) -> String {
    let mut items = String::new();
    for ch in &book.chapters {
        if let Some(p) = pages.get(ch.first_page) {
            items.push_str(&format!(
                "      <li><a href=\"text/{}\">{}</a></li>\n",
                p.xhtml_name,
                escape(&ch.title)
            ));
        }
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>{title}</title></head>
<body>
  <nav epub:type="toc" id="toc">
    <h1>{title}</h1>
    <ol>
{items}    </ol>
  </nav>
</body>
</html>
"#,
        title = escape(&book.title),
    )
}
