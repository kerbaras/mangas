//! Selector-based extraction for providers that scrape HTML. Pure functions over a parsed
//! body: parse once with [`parse`], then [`select`] / [`first`] from [`root`] or any element and
//! read values with [`text`], [`attr`], [`meta`] and [`absolute`].

use mangas_core::domain::{Error, Result};
use scraper::{Html, Selector};
use url::Url;

pub use scraper::{ElementRef as Element, Html as Document};

/// Parses a full page or a fragment (HTMX responses); fragments get an implied `<html>`.
pub fn parse(body: &str) -> Document {
    Html::parse_document(body)
}

/// The element every selection of a document starts from.
pub fn root(doc: &Document) -> Element<'_> {
    doc.root_element()
}

/// A compiled CSS selector; an invalid one is a provider bug reported as [`Error::Source`].
pub fn selector(css: &str) -> Result<Selector> {
    Selector::parse(css).map_err(|e| Error::Source {
        name: "html".to_string(),
        message: format!("invalid selector {css:?}: {e}"),
    })
}

/// Every descendant of `scope` matching `css`, in document order.
pub fn select<'a>(scope: Element<'a>, css: &str) -> Result<Vec<Element<'a>>> {
    let selector = selector(css)?;
    Ok(scope.select(&selector).collect())
}

/// The first descendant of `scope` matching `css`.
pub fn first<'a>(scope: Element<'a>, css: &str) -> Result<Option<Element<'a>>> {
    let selector = selector(css)?;
    Ok(scope.select(&selector).next())
}

/// The text of `element` with whitespace runs collapsed to one space and trimmed.
pub fn text(element: Element<'_>) -> String {
    collapse(&element.text().collect::<String>())
}

/// Like [`text`], keeping line breaks (descriptions with paragraphs); blank lines collapse.
pub fn text_lines(element: Element<'_>) -> String {
    let raw: String = element.text().collect();
    raw.lines()
        .map(collapse)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The trimmed value of attribute `name`, if present and not blank.
pub fn attr(element: Element<'_>, name: &str) -> Option<String> {
    let value = element.value().attr(name)?.trim();
    Some(value.to_string()).filter(|v| !v.is_empty())
}

/// `content` of `<meta property="…">` (Open Graph) or `<meta name="…">`.
pub fn meta(doc: &Document, key: &str) -> Result<Option<String>> {
    let css = format!(r#"meta[property="{key}"], meta[name="{key}"]"#);
    Ok(first(root(doc), &css)?.and_then(|m| attr(m, "content")))
}

/// `href` resolved against `base` (a page URL or origin); `None` when it does not parse.
pub fn absolute(base: &str, href: &str) -> Option<String> {
    let url = Url::parse(base).ok()?.join(href.trim()).ok()?;
    matches!(url.scheme(), "http" | "https").then(|| url.to_string())
}

fn collapse(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}
