//! The navigation document: one TOC entry per chapter plus cover/start landmarks.

use super::xml::escape;
use super::{Book, PageEntry, COVER_XHTML};

fn toc_items(book: &Book, pages: &[PageEntry]) -> String {
    book.chapters
        .iter()
        .filter_map(|ch| {
            pages.get(ch.first_page).map(|p| {
                format!(
                    "      <li><a href=\"text/{}\">{}</a></li>\n",
                    p.xhtml_name,
                    escape(&ch.title)
                )
            })
        })
        .collect()
}

fn landmarks(book: &Book, pages: &[PageEntry]) -> String {
    let cover = match (&book.cover, pages.first()) {
        (Some(_), _) => Some(COVER_XHTML.to_string()),
        (None, Some(p)) => Some(p.xhtml_name.clone()),
        (None, None) => None,
    };
    let mut items = String::new();
    if let Some(href) = cover {
        items.push_str(&format!(
            "      <li><a epub:type=\"cover\" href=\"text/{href}\">Cover</a></li>\n"
        ));
    }
    if let Some(p) = pages.first() {
        items.push_str(&format!(
            "      <li><a epub:type=\"bodymatter\" href=\"text/{}\">Start</a></li>\n",
            p.xhtml_name
        ));
    }
    items
}

pub fn nav_xhtml(book: &Book, pages: &[PageEntry]) -> String {
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
  <nav epub:type="landmarks" id="landmarks" hidden="hidden">
    <ol>
{landmarks}    </ol>
  </nav>
</body>
</html>
"#,
        title = escape(&book.title),
        items = toc_items(book, pages),
        landmarks = landmarks(book, pages),
    )
}
