//! Kindles on the USB bus, mounted or not: Amazon's vendor id over `nusb` (pure Rust, no
//! libusb). A Kindle that is on the bus but has no mounted folder talks MTP only.

use serde::Serialize;

/// Amazon's USB vendor id, shared by every Kindle model (product ids are not listed on purpose).
pub const AMAZON_VENDOR_ID: u16 = 0x1949;

/// QA hook: a fake Kindle on the bus with this product name, for systems without one.
const FAKE_DEVICE: &str = "MANGAS_FAKE_USB_KINDLE";

/// A Kindle seen on the USB bus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsbKindle {
    /// The product string it reports ("Kindle" when it reports none).
    pub name: String,
}

/// The Kindles on the USB bus now. An unreadable bus counts as none.
pub fn usb_kindles() -> Vec<UsbKindle> {
    if let Some(name) = std::env::var_os(FAKE_DEVICE).filter(|n| !n.is_empty()) {
        return vec![UsbKindle {
            name: name.to_string_lossy().into_owned(),
        }];
    }
    listed().unwrap_or_else(|e| {
        tracing::warn!("could not list USB devices: {e}");
        Vec::new()
    })
}

fn listed() -> Result<Vec<UsbKindle>, nusb::Error> {
    use nusb::MaybeFuture;
    Ok(nusb::list_devices()
        .wait()?
        .filter(|d| d.vendor_id() == AMAZON_VENDOR_ID)
        .map(|d| UsbKindle {
            name: d.product_string().unwrap_or("Kindle").to_string(),
        })
        .collect())
}
