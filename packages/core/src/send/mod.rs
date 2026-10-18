//! Delivery of a finished export to a Kindle: by email to its Send to Kindle address, or by
//! copying the file onto a Kindle mounted over USB.

pub mod email;
pub mod usb;

use serde::{Deserialize, Serialize};

pub use email::{EmailSettings, Security};
pub use usb::KindleVolume;

/// How a book reached the device, as recorded in the export history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SendVia {
    Email,
    Usb,
}

impl SendVia {
    pub fn as_str(self) -> &'static str {
        match self {
            SendVia::Email => "email",
            SendVia::Usb => "usb",
        }
    }

    pub fn parse(s: &str) -> Option<SendVia> {
        match s {
            "email" => Some(SendVia::Email),
            "usb" => Some(SendVia::Usb),
            _ => None,
        }
    }
}
