//! A Kindle on the USB bus with no mounted folder (MTP only) versus a mounted one or none.

use mangas_core::send::usb::{choose, decide, KindleScan, KindleVolume, Reach, UsbKindle};

fn volume(root: &str) -> KindleVolume {
    KindleVolume {
        name: "Kindle".into(),
        root: root.into(),
        documents: format!("{root}/documents").into(),
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
        "Your Kindle is connected over MTP; copying over USB isn't supported on this system — use Email delivery"
    );
    let err = choose(None, KindleScan::default()).unwrap_err().to_string();
    assert!(
        err.starts_with("Device not found: connect your Kindle"),
        "{err}"
    );
}
