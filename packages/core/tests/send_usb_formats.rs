use std::path::Path;

use mangas_core::export::Format;
use mangas_core::send::usb::{admit, sideload, usb_verdict, Sideload, SIDELOAD};
use mangas_core::Error;

#[test]
fn the_table_says_what_a_kindle_opens_side_loaded() {
    assert_eq!(sideload("pdf"), Some(Sideload::Supported));
    assert_eq!(sideload("azw3"), Some(Sideload::Supported));
    assert_eq!(sideload("EPUB"), Some(Sideload::Warn));
    assert_eq!(sideload("cbz"), Some(Sideload::Refused));
    assert_eq!(sideload("zip"), None);
}

#[test]
fn every_export_format_has_an_entry() {
    for f in Format::ALL {
        assert!(
            sideload(f.id()).is_some(),
            "{} missing from SIDELOAD",
            f.id()
        );
    }
    assert_eq!(SIDELOAD.len(), 4);
}

#[test]
fn a_pdf_or_azw3_is_admitted_without_a_reason() {
    for file in ["Vol 1.pdf", "Vol 1.AZW3"] {
        let v = usb_verdict(Path::new(file), "Kindle Paperwhite");
        assert_eq!(v.decision, Sideload::Supported, "{file}");
        assert_eq!(v.reason, None);
        admit(&v, false).unwrap();
    }
}

#[test]
fn a_cbz_is_refused_even_when_forced_naming_the_device_and_a_pdf_export() {
    let v = usb_verdict(Path::new("/b/Vol 1.cbz"), "Kindle Paperwhite");
    assert_eq!(v.decision, Sideload::Refused);
    assert_eq!(v.format, "CBZ");
    let reason = v.reason.clone().unwrap();
    assert!(
        reason.starts_with("Kindle Paperwhite doesn't open CBZ files"),
        "{reason}"
    );
    assert!(reason.contains("PDF or AZW3"), "{reason}");
    for force in [false, true] {
        let err = admit(&v, force).unwrap_err();
        assert!(matches!(err, Error::NotOnKindle(_)));
        assert_eq!(err.to_string(), reason);
    }
}

#[test]
fn an_epub_warns_points_to_email_and_goes_through_once_confirmed() {
    let v = usb_verdict(Path::new("/b/Vol 1.epub"), "Kindle");
    assert_eq!(v.decision, Sideload::Warn);
    assert_eq!(v.format, "EPUB");
    let reason = v.reason.clone().unwrap();
    assert!(
        reason.starts_with("Kindle may not open an EPUB"),
        "{reason}"
    );
    assert!(reason.contains("Email it instead"), "{reason}");
    assert!(matches!(admit(&v, false), Err(Error::NotOnKindle(_))));
    admit(&v, true).unwrap();
}

#[test]
fn a_file_that_is_no_export_format_warns() {
    let v = usb_verdict(Path::new("notes.txt"), "Kindle");
    assert_eq!(v.decision, Sideload::Warn);
    assert!(v.reason.unwrap().contains("a .txt file"));
    let bare = usb_verdict(Path::new("README"), "Kindle");
    assert_eq!(bare.decision, Sideload::Warn);
    assert!(bare.reason.unwrap().contains("without an extension"));
}

#[test]
fn the_verdict_serialises_with_a_stable_shape() {
    let v = usb_verdict(Path::new("a.cbz"), "Kindle");
    let json = serde_json::to_value(&v).unwrap();
    assert_eq!(json["decision"], "refused");
    assert_eq!(json["format"], "CBZ");
    assert_eq!(json["device"], "Kindle");
    assert!(json["reason"].is_string());
    let ok = serde_json::to_value(usb_verdict(Path::new("a.pdf"), "Kindle")).unwrap();
    assert_eq!(ok["decision"], "supported");
    assert!(ok["reason"].is_null());
}
