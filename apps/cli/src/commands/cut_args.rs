//! Device, smart-cut and detector flags shared by the export commands.

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::export::kindle::{device, Device, Progress};
use mangas_core::export::{ExportPreset, Format};
use mangas_core::ml::{self, DetectorReport};
use mangas_core::smartcut::{Preset, RowPenalizer};

use crate::paths;

/// The profile when neither `--profile` nor a stored default names one.
pub const DEFAULT_PROFILE: &str = "kindle-paperwhite12";

#[derive(ClapArgs)]
pub struct CutArgs {
    /// Target e-reader profile id (see `mangas kindle --list-profiles`)
    #[arg(long, visible_alias = "profile")]
    pub device: Option<String>,
    /// Smart-cut preset for long strips [default: the series preset, else balanced]
    #[arg(long, value_enum)]
    pub cut_preset: Option<PresetArg>,
    /// Override the preset's max shrink (1.0 = never shrink, 1.25 = up to 25% more rows per page)
    #[arg(long)]
    pub max_shrink: Option<f64>,
    /// Override the preset's overlap after a forced cut (fraction of a page, e.g. 0.075)
    #[arg(long)]
    pub overlap: Option<f64>,
    /// Height of a page cut from a long strip, as a fraction of the screen (0.5–1.0)
    #[arg(long, value_name = "FRACTION")]
    pub page_height: Option<f64>,
    /// Crop blank page borders so the art fills more of the screen (`--trim-margins=false` to
    /// turn a series preset's trim off)
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub trim_margins: Option<bool>,
    /// E-ink contrast factor (0.5–2.0) [default: the profile's]
    #[arg(long)]
    pub contrast: Option<f64>,
    /// E-ink gamma (0.5–2.0) [default: the profile's]
    #[arg(long)]
    pub gamma: Option<f64>,
    /// Sharpen grey pages [default: the profile's]
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub sharpen: Option<bool>,
    /// JPEG quality of the pages (40–95) [default: the profile's, 90 at 300 dpi]; lower makes
    /// smaller books
    #[arg(long, value_name = "40-95")]
    pub quality: Option<u8>,
    /// Write a book bigger than this many MB as numbered parts, "Title (part 1).epub", ... (45
    /// keeps each part under the 50 MB Send-to-Kindle email limit)
    #[arg(long, value_name = "MB", value_parser = clap::value_parser!(u64).range(1..))]
    pub split_mb: Option<u64>,
    /// Run the ONNX bubble/text/face detectors (needs a build with the `ml` feature). When they
    /// can't run, the export warns and cuts without them (exit 0)
    #[arg(long)]
    pub detectors: bool,
    /// With --detectors: when the detectors can't run, fail with exit 1 and the runtime's error
    /// before anything is written, instead of cutting without them
    #[arg(long, requires = "detectors")]
    pub strict_detectors: bool,
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum PresetArg {
    Balanced,
    FewestCuts,
    NoShrink,
}

impl From<PresetArg> for Preset {
    fn from(p: PresetArg) -> Self {
        match p {
            PresetArg::Balanced => Preset::Balanced,
            PresetArg::FewestCuts => Preset::FewestCuts,
            PresetArg::NoShrink => Preset::NoShrink,
        }
    }
}

impl CutArgs {
    /// The resolved preset's profile, else `fallback`.
    pub fn device_of(preset: &ExportPreset, fallback: &str) -> anyhow::Result<Device> {
        let id = preset.profile.as_deref().unwrap_or(fallback);
        match device(id) {
            Some(d) => Ok(d),
            None => bail!("unknown profile '{id}' (try --list-profiles)"),
        }
    }

    /// The `--split-mb` part size in bytes.
    pub fn split(&self) -> Option<u64> {
        self.split_mb.map(|mb| mb * 1_000_000)
    }

    /// The options given on the command line, as the top layer over a series preset.
    pub fn explicit(&self, format: Option<Format>) -> anyhow::Result<ExportPreset> {
        let preset = ExportPreset {
            cut_preset: self.cut_preset.map(Preset::from),
            max_shrink: self.max_shrink,
            overlap: self.overlap,
            page_height: self.page_height,
            trim_margins: self.trim_margins,
            contrast: self.contrast,
            gamma: self.gamma,
            sharpen: self.sharpen,
            jpeg_quality: self.quality,
            format,
            profile: self.device.clone(),
        };
        preset.validate()?;
        Ok(preset)
    }

    /// The detector-backed row penalizer when `--detectors` is set, downloading models first,
    /// and what became of the detectors. Detectors that can't run are reported and left off, so
    /// the cut goes on without them; with `--strict-detectors` that is an error instead.
    pub async fn penalizer(&self) -> anyhow::Result<(Option<RowPenalizer>, DetectorReport)> {
        if !self.detectors {
            return Ok((None, DetectorReport::off()));
        }
        let cache = paths::models()?;
        let missing = ml::status(&cache).download_bytes;
        if missing > 0 {
            eprintln!(
                "Detector support was missing; downloading {}…",
                mangas_core::storage::human_size(missing)
            );
        }
        let mut on_progress = |id: &str, done: u64, total: Option<u64>| {
            eprint!(
                "\rdownloading {id}: {done}/{}",
                total.map(|t| t.to_string()).unwrap_or_default()
            );
        };
        let (settings, warning) = mangas_core::library::settings_or_default(&paths::library_db()?);
        if let Some(warning) = warning {
            eprintln!("warning: {warning}");
        }
        let detection =
            ml::penalizer_or_reason(&cache, &settings.network(), &mut on_progress).await;
        if missing > 0 {
            eprintln!();
        }
        if let (Some(e), true) = (&detection.error, self.strict_detectors) {
            bail!("detectors were requested (--strict-detectors) but can't run: {e}")
        }
        let report = detection.report();
        let provider = match report.state {
            ml::DetectorState::Used => ml::status(&cache).execution_provider,
            _ => None,
        };
        let provider = provider.unwrap_or_else(|| "default".into());
        if let Some(line) = report.message(&provider) {
            eprintln!("{line}");
        }
        Ok((detection.penalizer, report))
    }
}

pub fn print_progress(p: Progress) {
    match p {
        Progress::Chapter { index, total, path } => {
            eprintln!("[{}/{}] {}", index + 1, total, path.display())
        }
        Progress::Pages { done } => eprint!("\r  pages: {done}"),
        Progress::Writing { pages } => eprintln!("\nwriting {pages} pages"),
    }
}
