//! Export presets: smart-cut options, page tweaks, format and profile, every field optional.
//! A series stores one; an export layers explicit options over it, and it over the defaults.

use serde::{Deserialize, Serialize};

use super::page::{PageSettings, JPEG_QUALITY, PAGE_HEIGHT, TONE};
use super::{profile, Format, Profile};
use crate::domain::{Error, Result};
use crate::library::Settings;
use crate::smartcut::{Options, Preset};

/// Allowed max shrink (1.0 = never shrink) and overlap (fraction of a page).
pub const MAX_SHRINK: (f64, f64) = (1.0, 1.5);
pub const OVERLAP: (f64, f64) = (0.0, 0.25);

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportPreset {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cut_preset: Option<Preset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_shrink: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlap: Option<f64>,
    /// Fraction of the screen height a cut page fills (see [`PageSettings::height`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trim_margins: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contrast: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gamma: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharpen: Option<bool>,
    /// JPEG quality of the pages ([`JPEG_QUALITY`]); lower makes smaller books.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jpeg_quality: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<Format>,
    /// Profile id (`export::PROFILES`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

macro_rules! layer {
    ($top:expr, $base:expr, $($f:ident),+) => {
        ExportPreset { $($f: $top.$f.clone().or_else(|| $base.$f.clone())),+ }
    };
}

impl ExportPreset {
    /// Each field of `self`, else the one of `base`.
    pub fn over(&self, base: &ExportPreset) -> ExportPreset {
        layer!(
            self,
            base,
            cut_preset,
            max_shrink,
            overlap,
            page_height,
            trim_margins,
            contrast,
            gamma,
            sharpen,
            jpeg_quality,
            format,
            profile
        )
    }

    /// Explicit options beat the series preset, which beats the fallback, field by field.
    pub fn resolve(
        explicit: &ExportPreset,
        series: Option<&ExportPreset>,
        fallback: &ExportPreset,
    ) -> ExportPreset {
        let below = series.map_or_else(|| fallback.clone(), |s| s.over(fallback));
        explicit.over(&below)
    }

    /// The stored defaults (cut preset, format, profile) as a fallback layer.
    pub fn from_settings(s: &Settings) -> ExportPreset {
        ExportPreset {
            cut_preset: s.default_preset,
            format: s.default_format,
            profile: s.default_device.clone(),
            ..Default::default()
        }
    }

    /// The quality stored for the preset's profile (else `default_profile`), when no layer set one.
    pub fn with_stored_quality(mut self, s: &Settings, default_profile: &str) -> ExportPreset {
        let id = self.profile.as_deref().unwrap_or(default_profile);
        self.jpeg_quality = self
            .jpeg_quality
            .or_else(|| s.jpeg_quality.get(id).copied());
        self
    }

    pub fn is_empty(&self) -> bool {
        *self == ExportPreset::default()
    }

    /// Smart-cut options: the cut preset (balanced when unset) with its overrides.
    pub fn cut(&self) -> Options {
        let mut opts = self.cut_preset.unwrap_or(Preset::Balanced).options();
        if let Some(s) = self.max_shrink.filter(|s| s.is_finite()) {
            opts.max_shrink = s.clamp(MAX_SHRINK.0, MAX_SHRINK.1);
        }
        if let Some(o) = self.overlap.filter(|o| o.is_finite()) {
            opts.overlap = o.clamp(OVERLAP.0, OVERLAP.1);
        }
        opts
    }

    pub fn page(&self) -> PageSettings {
        PageSettings {
            height: self.page_height,
            trim_margins: self.trim_margins.unwrap_or(false),
            contrast: self.contrast,
            gamma: self.gamma,
            sharpen: self.sharpen,
            quality: self.jpeg_quality,
        }
    }

    /// The preset's profile, else `default` (a profile id).
    pub fn profile_or(&self, default: &str) -> Result<Profile> {
        let id = self.profile.as_deref().unwrap_or(default);
        profile(id).ok_or_else(|| Error::Other(format!("unknown profile '{id}'")))
    }

    /// Refuses values outside the ranges the export accepts, naming the field.
    pub fn validate(&self) -> Result<()> {
        let checks = [
            ("max shrink", self.max_shrink, MAX_SHRINK),
            ("overlap", self.overlap, OVERLAP),
            ("page height", self.page_height, PAGE_HEIGHT),
            ("contrast", self.contrast, TONE),
            ("gamma", self.gamma, TONE),
        ];
        for (name, value, (lo, hi)) in checks {
            if let Some(v) = value.filter(|v| !(v.is_finite() && (lo..=hi).contains(v))) {
                return Err(Error::Other(format!("{name} {v} is outside {lo}–{hi}")));
            }
        }
        let (lo, hi) = JPEG_QUALITY;
        if let Some(q) = self.jpeg_quality.filter(|q| !(lo..=hi).contains(q)) {
            return Err(Error::Other(format!(
                "JPEG quality {q} is outside {lo}–{hi}"
            )));
        }
        if let Some(p) = &self.profile {
            self.profile_or(p)?;
        }
        Ok(())
    }
}
