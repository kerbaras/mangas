//! A Kindle on the USB bus with no mounted folder (MTP only) versus a mounted one or none.

use mangas_core::send::email::{email_refusal, emailable};
use mangas_core::send::usb::{
    choose, chosen_name, decide, KindleScan, KindleVolume, Reach, UsbKindle,
};

fn volume(root: &str) -> KindleVolume {
    KindleVolume {
        name: "Kindle".into(),
        root: root.into(),
        documents: format!("{root}/documents").into(),
        ejectable: true,
    }
}

#[test]
fn a_kindle_on_the_bus_without_a_folder_is_mtp_only() {
    let device = UsbKindle {
        name: "Kindle Paperwhite".into(),
    };
    let mounted = volume("/Volumes/Kindle");
    // No device.
    assert_eq!(decide(vec![], vec![]).reach(), Reach::Nothing);
    // Mass storage only.
    let only_drive = decide(vec![mounted.clone()], vec![]);
    assert_eq!(only_drive.reach(), Reach::Mounted(mounted.clone()));
    // MTP only: on the bus, nothing mounted.
    let mtp = decide(vec![], vec![device.clone()]);
    assert_eq!(mtp.mtp, std::slice::from_ref(&device));
    assert_eq!(mtp.clone().reach(), Reach::MtpOnly(device.clone()));
    // Both: a mounted Kindle is on the bus too; the folder wins and no MTP hint shows.
    let both = decide(vec![mounted.clone()], vec![device]);
    assert!(both.mtp.is_empty());
    assert_eq!(both.reach(), Reach::Mounted(mounted));
}

#[test]
fn choose_says_mtp_instead_of_device_not_found() {
    let device = UsbKindle {
        name: "Kindle".into(),
    };
    let err = choose(None, decide(vec![], vec![device])).unwrap_err();
    assert!(matches!(err, mangas_core::Error::KindleOverMtp { .. }));
    assert_eq!(
        err.to_string(),
        "Kindle connects over MTP, which mangas can't copy to on this computer yet"
    );
    let err = choose(None, KindleScan::default()).unwrap_err().to_string();
    assert!(
        err.starts_with("Device not found: connect your Kindle"),
        "{err}"
    );
}

#[test]
fn the_format_table_hears_the_name_of_a_kindle_on_mtp_only() {
    let device = UsbKindle {
        name: "Kindle Paperwhite".into(),
    };
    let mtp = choose(None, decide(vec![], vec![device]));
    assert_eq!(chosen_name(&mtp), Some("Kindle Paperwhite"));
    let mounted = choose(None, decide(vec![volume("/Volumes/Kindle")], vec![]));
    assert_eq!(chosen_name(&mounted), Some("Kindle"));
    assert_eq!(chosen_name(&choose(None, KindleScan::default())), None);
}

#[test]
fn email_takes_epub_and_pdf_only() {
    for (file, takes) in [
        ("a.epub", true),
        ("B.PDF", true),
        ("c.cbz", false),
        ("d.azw3", false),
        ("e", false),
    ] {
        assert_eq!(emailable(file), takes, "{file}");
        assert_eq!(email_refusal(file).is_none(), takes, "{file}");
    }
    assert_eq!(
        email_refusal("/b/Vol 1.cbz").as_deref(),
        Some("Send to Kindle by email takes EPUB or PDF, not a CBZ. Copy it over USB, or export it as EPUB.")
    );
    assert!(email_refusal("/b/v1.0/book").is_some_and(|r| r.contains("not this file")));
}
