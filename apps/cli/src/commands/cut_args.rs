//! Device, smart-cut and detector flags shared by the export commands.

use anyhow::{bail, Context};
use clap::Args as ClapArgs;
use mangas_core::export::kindle::{device, Device, Progress};
use mangas_core::ml;
use mangas_core::smartcut::{Options, Preset, RowPenalizer};

use crate::paths;

/// The profile when neither `--profile` nor a stored default names one.
pub const DEFAULT_PROFILE: &str = "kindle-paperwhite12";

#[derive(ClapArgs)]
pub struct CutArgs {
    /// Target e-reader profile id (see `mangas kindle --list-profiles`)
    #[arg(long, visible_alias = "profile")]
    pub device: Option<String>,
    /// Smart-cut preset for long strips
    #[arg(long, value_enum, default_value_t = PresetArg::Balanced)]
    pub cut_preset: PresetArg,
    /// Override the preset's max shrink (1.0 = never shrink, 1.25 = up to 25% more rows per page)
    #[arg(long)]
    pub max_shrink: Option<f64>,
    /// Override the preset's overlap after a forced cut (fraction of a page, e.g. 0.075)
    #[arg(long)]
    pub overlap: Option<f64>,
    /// Run the ONNX bubble/text/face detectors (needs a build with the `ml` feature)
    #[arg(long)]
    pub detectors: bool,
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
    /// The `--profile` given, else `fallback`.
    pub fn device_or(&self, fallback: &str) -> anyhow::Result<Device> {
        let id = self.device.as_deref().unwrap_or(fallback);
        match device(id) {
            Some(d) => Ok(d),
            None => bail!("unknown profile '{id}' (try --list-profiles)"),
        }
    }

    pub fn options(&self) -> Options {
        let mut opts = Preset::from(self.cut_preset).options();
        if let Some(s) = self.max_shrink {
            opts.max_shrink = s;
        }
        if let Some(o) = self.overlap {
            opts.overlap = o;
        }
        opts
    }

    /// The detector-backed row penalizer when `--detectors` is set, downloading models first.
    pub async fn penalizer(&self) -> anyhow::Result<Option<RowPenalizer>> {
        if !self.detectors {
            return Ok(None);
        }
        let cache = paths::models()?;
        let mut on_progress = |id: &str, done: u64, total: Option<u64>| {
            eprint!(
                "\rdownloading {id}: {done}/{}",
                total.map(|t| t.to_string()).unwrap_or_default()
            );
        };
        let p = ml::row_penalizer(&cache, &mut on_progress)
            .await
            .context("detectors")?;
        eprintln!("detectors: {:?}", ml::status(&cache).execution_provider);
        Ok(Some(p))
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
