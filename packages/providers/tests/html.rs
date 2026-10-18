//! The shared HTML helper: selection, text, attributes, meta tags and URL resolution.

use mangas_providers::html::{
    absolute, attr, decode_entities, first, meta, parse, root, select, selector, text, text_lines,
};

const PAGE: &str = r#"<!DOCTYPE html><html><head>
<meta property="og:image" content=" https://cdn.example.com/c.jpg ">
<meta name="description" content="About &amp; more"></head>
<body><ul>
  <li class="x"><a href="/a/1">  One
     title </a></li>
  <li class="x"><a href="b/2" data-tip="">Two</a></li>
</ul><p>line one
   <br>
line   two</p></body></html>"#;

#[test]
fn select_returns_matches_in_document_order() {
    let doc = parse(PAGE);
    let links = select(root(&doc), "li.x a").unwrap();
    let texts: Vec<_> = links.iter().map(|a| text(*a)).collect();
    assert_eq!(texts, ["One title", "Two"]);
    let li = select(root(&doc), "li").unwrap()[1];
    assert_eq!(first(li, "a").unwrap().map(text).as_deref(), Some("Two"));
    assert!(first(li, "p").unwrap().is_none());
}

#[test]
fn fragments_parse_like_documents() {
    let doc = parse(r#"<article><a href="/s/1">A</a></article><article></article>"#);
    assert_eq!(select(root(&doc), "article").unwrap().len(), 2);
}

#[test]
fn attributes_are_trimmed_and_blank_ones_missing() {
    let doc = parse(PAGE);
    let links = select(root(&doc), "a").unwrap();
    assert_eq!(attr(links[0], "href").as_deref(), Some("/a/1"));
    assert_eq!(attr(links[1], "data-tip"), None);
    assert_eq!(attr(links[1], "missing"), None);
}

#[test]
fn meta_reads_property_and_name_tags_unescaped() {
    let doc = parse(PAGE);
    let image = meta(&doc, "og:image").unwrap();
    assert_eq!(image.as_deref(), Some("https://cdn.example.com/c.jpg"));
    assert_eq!(
        meta(&doc, "description").unwrap().as_deref(),
        Some("About & more")
    );
    assert_eq!(meta(&doc, "og:title").unwrap(), None);
}

#[test]
fn text_lines_keeps_line_breaks() {
    let doc = parse(PAGE);
    let p = first(root(&doc), "p").unwrap().unwrap();
    assert_eq!(text_lines(p), "line one\nline two");
}

#[test]
fn absolute_resolves_relative_links_and_rejects_other_schemes() {
    let base = "https://example.com/series/1";
    assert_eq!(
        absolute(base, "/a/1").as_deref(),
        Some("https://example.com/a/1")
    );
    assert_eq!(
        absolute(base, "b/2").as_deref(),
        Some("https://example.com/series/b/2")
    );
    let cdn = "https://cdn.example.com/x.png";
    assert_eq!(absolute(base, cdn).as_deref(), Some(cdn));
    assert_eq!(absolute(base, "javascript:alert(1)"), None);
    assert_eq!(absolute("not a url", "/a"), None);
}

#[test]
fn invalid_selectors_are_errors_not_panics() {
    assert!(selector("a[").is_err());
    let doc = parse(PAGE);
    assert!(select(root(&doc), "::nope").is_err());
}

#[test]
fn entities_decode_named_and_numeric_references() {
    let cases = [
        ("Ca&aacute;n", "Caán"),
        ("&iquest;Por qu&eacute;?", "¿Por qué?"),
        ("It&rsquo;s here &mdash; finally", "It’s here — finally"),
        ("Tom &amp; Jerry", "Tom & Jerry"),
        ("Don&#39;t / Don&#x27;t", "Don't / Don't"),
        ("&bogus; & plain", "&bogus; & plain"),
    ];
    for (raw, decoded) in cases {
        assert_eq!(decode_entities(raw), decoded, "{raw}");
    }
}
