//! Page tweaks on top of a device profile: shorter cut pages, margin trim and e-ink filters.
//! The default changes nothing, so an export without tweaks matches the profile exactly.

use serde::{Deserialize, Serialize};

use super::kindle::{Device, ImageSettings};

/// Allowed page heights, as a fraction of the screen height.
pub const PAGE_HEIGHT: (f64, f64) = (0.5, 1.0);
/// Allowed contrast and gamma factors.
pub const TONE: (f64, f64) = (0.5, 2.0);

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PageSettings {
    /// Height of a page cut from a long strip, as a fraction of the screen; `None` is all of it.
    pub height: Option<f64>,
    /// Crop blank borders so the art fills more of the screen.
    pub trim_margins: bool,
    /// Filter overrides; `None` keeps what the profile recommends.
    pub contrast: Option<f64>,
    pub gamma: Option<f64>,
    pub sharpen: Option<bool>,
}

fn within(v: f64, (lo, hi): (f64, f64)) -> f64 {
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        hi
    }
}

impl PageSettings {
    /// Pixel height of a cut page on `device`.
    pub fn page_height(&self, device: &Device) -> u32 {
        let f = self.height.map_or(1.0, |h| within(h, PAGE_HEIGHT));
        ((device.height as f64 * f).round() as u32).max(1)
    }

    /// The profile's filters with the overrides applied.
    pub fn image_settings(&self, device: &Device) -> ImageSettings {
        let base = ImageSettings::for_device(device);
        ImageSettings {
            contrast: self.contrast.map_or(base.contrast, |c| within(c, TONE)),
            gamma: self.gamma.map_or(base.gamma, |g| within(g, TONE)),
            sharpen: self.sharpen.unwrap_or(base.sharpen),
            ..base
        }
    }
}
