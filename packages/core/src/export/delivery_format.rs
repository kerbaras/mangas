//! The format a book goes out in for its delivery. A Kindle opens AZW3 copied over USB but not
//! EPUB, while Send to Kindle by email takes EPUB (and converts it) but not AZW3, so a Kindle
//! profile's format follows the delivery unless the user picked one for this export.

use serde::Serialize;

use super::{Format, Profile};

/// How the book leaves the computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Save,
    Usb,
    Email,
}

/// Why a Kindle book is AZW3 or EPUB, as the plan and the Export dialog state it.
pub const KINDLE_REASON: &str = "Kindle over USB: AZW3; Email: EPUB";

/// The formats a profile takes per delivery, when they differ from its own `format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DeliveryFormats {
    pub usb: Format,
    pub email: Format,
    pub reason: &'static str,
}

/// A device profile with its formats per delivery, as the desktop lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DeviceFormats {
    #[serde(flatten)]
    pub profile: Profile,
    pub delivery: Option<DeliveryFormats>,
}

/// The chosen format and, when the delivery decided it, why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Chosen {
    pub format: Format,
    pub reason: Option<&'static str>,
}

/// AZW3 over USB and EPUB by email for a Kindle; `None` for other e-readers.
pub fn delivery_formats(profile: &Profile) -> Option<DeliveryFormats> {
    profile
        .id
        .starts_with("kindle-")
        .then_some(DeliveryFormats {
            usb: Format::Azw3,
            email: Format::Epub,
            reason: KINDLE_REASON,
        })
}

/// `profile` with [`delivery_formats`].
pub fn device_formats(profile: Profile) -> DeviceFormats {
    DeviceFormats {
        delivery: delivery_formats(&profile),
        profile,
    }
}

/// The format of an export to `profile` over `route`: `picked` (a flag or a pick in the dialog)
/// wins; else the delivery's format for a Kindle sent over USB or email; else `stored` (the
/// series preset or the default), else the profile's own format.
pub fn for_delivery(
    picked: Option<Format>,
    stored: Option<Format>,
    profile: &Profile,
    route: Route,
) -> Chosen {
    let by_route = delivery_formats(profile).and_then(|d| match route {
        Route::Save => None,
        Route::Usb => Some((d.usb, d.reason)),
        Route::Email => Some((d.email, d.reason)),
    });
    match (picked, by_route) {
        (Some(format), _) => Chosen {
            format,
            reason: None,
        },
        (None, Some((format, reason))) => Chosen {
            format,
            reason: Some(reason),
        },
        (None, None) => Chosen {
            format: stored.unwrap_or(profile.format),
            reason: None,
        },
    }
}

#[cfg(test)]
mod tests;
