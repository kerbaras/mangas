//! E-reader profiles: screen size, density, colour and the file format each device reads best.
//! Plain data; ids are stored in the settings (`default_device`), so never rename one.

use serde::{Deserialize, Serialize};

use super::Format;

/// Screen of a target e-reader in pixels, and its recommended export format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub id: &'static str,
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub grayscale: bool,
    pub format: Format,
}

const fn p(
    id: &'static str,
    name: &'static str,
    (width, height): (u32, u32),
    dpi: u32,
    grayscale: bool,
    format: Format,
) -> Profile {
    Profile {
        id,
        name,
        width,
        height,
        dpi,
        grayscale,
        format,
    }
}

use Format::{Cbz, Epub, Pdf};

#[rustfmt::skip]
pub const PROFILES: &[Profile] = &[
    p("kindle-paperwhite12", "Kindle Paperwhite (12th gen) / Signature", (1264, 1680), 300, true, Epub),
    p("kindle-paperwhite5", "Kindle Paperwhite (11th gen) / Signature", (1236, 1648), 300, true, Epub),
    p("kindle-paperwhite3", "Kindle Paperwhite 3/4", (1072, 1448), 300, true, Epub),
    p("kindle-scribe", "Kindle Scribe", (1860, 2480), 300, true, Epub),
    p("kindle-oasis3", "Kindle Oasis 3", (1264, 1680), 300, true, Epub),
    p("kobo-clara", "Kobo Clara BW / 2E / HD", (1072, 1448), 300, true, Cbz),
    p("kobo-clara-colour", "Kobo Clara Colour", (1072, 1448), 300, false, Cbz),
    p("kobo-libra", "Kobo Libra 2 / H2O", (1264, 1680), 300, true, Cbz),
    p("kobo-libra-colour", "Kobo Libra Colour", (1264, 1680), 300, false, Cbz),
    p("kobo-elipsa", "Kobo Elipsa 2E", (1404, 1872), 227, true, Cbz),
    p("remarkable2", "reMarkable 2", (1404, 1872), 226, true, Pdf),
    p("tablet", "Generic tablet (colour)", (1536, 2048), 264, false, Cbz),
];

pub fn profile(id: &str) -> Option<Profile> {
    PROFILES.iter().copied().find(|d| d.id == id)
}
