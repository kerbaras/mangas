//! The e-reader profile table and how an export picks its format.

use std::collections::HashSet;
use std::path::Path;

use mangas_core::export::{kindle, profile, Format, PROFILES};

#[test]
fn ids_are_unique_and_geometry_is_sane() {
    let ids: HashSet<_> = PROFILES.iter().map(|p| p.id).collect();
    assert_eq!(ids.len(), PROFILES.len());
    for p in PROFILES {
        assert!(p.height > p.width, "{} is portrait", p.id);
        assert!((150..=400).contains(&p.dpi), "{} dpi", p.id);
        assert!(!p.name.is_empty());
    }
}

#[test]
fn required_devices_are_listed() {
    for id in [
        "kindle-paperwhite12",
        "kindle-scribe",
        "kobo-clara",
        "kobo-libra",
        "tablet",
    ] {
        assert!(profile(id).is_some(), "{id}");
    }
    assert_eq!(
        profile("kindle-scribe").map(|p| (p.width, p.height)),
        Some((1860, 2480))
    );
    assert!(profile("tablet").is_some_and(|p| !p.grayscale));
    assert!(profile("nope").is_none());
}

#[test]
fn stored_kindle_ids_still_resolve() {
    for id in [
        "kindle-paperwhite12",
        "kindle-paperwhite5",
        "kindle-paperwhite3",
        "kindle-oasis3",
    ] {
        assert_eq!(kindle::device(id), profile(id));
    }
    assert!(PROFILES
        .iter()
        .filter(|p| p.id.starts_with("kindle"))
        .all(|p| p.format == Format::Epub));
}

#[test]
fn format_parses_case_insensitively() {
    assert_eq!("PDF".parse::<Format>().unwrap(), Format::Pdf);
    assert_eq!(" cbz ".parse::<Format>().unwrap(), Format::Cbz);
    assert!("mobi".parse::<Format>().is_err());
    assert_eq!(Format::Epub.exporter().name(), "EPUB");
}

#[test]
fn format_resolution_order() {
    let kobo = profile("kobo-clara").unwrap();
    let pdf = Some(Path::new("out/Book.PDF"));
    assert_eq!(
        Format::resolve(Some(Format::Epub), None, Some(Format::Pdf), &kobo).unwrap(),
        Format::Epub
    );
    assert_eq!(
        Format::resolve(None, pdf, Some(Format::Epub), &kobo).unwrap(),
        Format::Pdf
    );
    assert_eq!(
        Format::resolve(None, Some(Path::new("out/dir")), Some(Format::Pdf), &kobo).unwrap(),
        Format::Pdf
    );
    assert_eq!(
        Format::resolve(None, None, None, &kobo).unwrap(),
        Format::Cbz
    );
    assert!(Format::resolve(Some(Format::Cbz), pdf, None, &kobo).is_err());
}

#[test]
fn default_format_is_stored_as_lowercase() {
    let s = mangas_core::library::Settings {
        default_format: Some(Format::Cbz),
        ..Default::default()
    };
    let json = serde_json::to_value(&s).unwrap();
    assert_eq!(json["default_format"], "cbz");
}
