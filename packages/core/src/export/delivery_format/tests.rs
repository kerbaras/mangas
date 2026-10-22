use super::*;
use crate::export::profile;

fn kindle() -> Profile {
    profile("kindle-paperwhite12").unwrap()
}

#[test]
fn a_kindle_gets_azw3_over_usb_and_epub_by_email_with_the_reason() {
    let usb = for_delivery(None, Some(Format::Pdf), &kindle(), Route::Usb);
    assert_eq!(usb.format, Format::Azw3);
    assert_eq!(usb.reason, Some(KINDLE_REASON));
    let email = for_delivery(None, Some(Format::Azw3), &kindle(), Route::Email);
    assert_eq!(email.format, Format::Epub);
    assert_eq!(email.reason, Some(KINDLE_REASON));
}

#[test]
fn saving_keeps_the_stored_format_else_the_profiles() {
    let saved = for_delivery(None, Some(Format::Pdf), &kindle(), Route::Save);
    assert_eq!((saved.format, saved.reason), (Format::Pdf, None));
    let plain = for_delivery(None, None, &kindle(), Route::Save);
    assert_eq!(plain.format, Format::Epub);
}

#[test]
fn a_picked_format_wins_over_the_delivery() {
    let picked = for_delivery(Some(Format::Epub), None, &kindle(), Route::Usb);
    assert_eq!((picked.format, picked.reason), (Format::Epub, None));
}

#[test]
fn other_readers_ignore_the_delivery() {
    let kobo = profile("kobo-clara").unwrap();
    assert_eq!(delivery_formats(&kobo), None);
    let usb = for_delivery(None, None, &kobo, Route::Usb);
    assert_eq!((usb.format, usb.reason), (Format::Cbz, None));
}

#[test]
fn every_kindle_profile_has_delivery_formats() {
    for p in crate::export::PROFILES
        .iter()
        .filter(|p| p.name.contains("Kindle"))
    {
        assert!(delivery_formats(p).is_some(), "{}", p.id);
    }
}
