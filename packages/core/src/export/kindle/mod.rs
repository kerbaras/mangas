//! Kindle export: device profiles, e-ink filters and the export pipeline.

mod cover;
mod export;
pub mod filters;
mod input;
pub mod process;
mod request;

use serde::{Deserialize, Serialize};

pub use export::export;
pub use input::{chapter_files, chapter_images, decode, ChapterFiles};
pub use process::{process_page, EncodedPage, ImageSettings};
pub use request::{BookMeta, ExportRequest, ExportStats, Progress};

/// Screen of a target e-reader in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub id: &'static str,
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub grayscale: bool,
}

pub const DEVICES: &[Device] = &[
    Device {
        id: "kindle-paperwhite12",
        name: "Kindle Paperwhite (12th gen) / Signature",
        width: 1264,
        height: 1680,
        dpi: 300,
        grayscale: true,
    },
    Device {
        id: "kindle-paperwhite5",
        name: "Kindle Paperwhite (11th gen) / Signature",
        width: 1236,
        height: 1648,
        dpi: 300,
        grayscale: true,
    },
    Device {
        id: "kindle-paperwhite3",
        name: "Kindle Paperwhite 3/4",
        width: 1072,
        height: 1448,
        dpi: 300,
        grayscale: true,
    },
    Device {
        id: "kindle-scribe",
        name: "Kindle Scribe",
        width: 1860,
        height: 2480,
        dpi: 300,
        grayscale: true,
    },
    Device {
        id: "kindle-oasis3",
        name: "Kindle Oasis 3",
        width: 1264,
        height: 1680,
        dpi: 300,
        grayscale: true,
    },
];

pub fn device(id: &str) -> Option<Device> {
    DEVICES.iter().copied().find(|d| d.id == id)
}
