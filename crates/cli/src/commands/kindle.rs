use std::path::PathBuf;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::export::kindle::{device, DEVICES};
use mangas_core::smartcut::Preset;

#[derive(ClapArgs)]
pub struct Args {
    /// Directories of downloaded chapters, in reading order
    pub chapters: Vec<PathBuf>,
    /// Target device id (see --list-devices)
    #[arg(long, default_value = "kindle-paperwhite12")]
    pub device: String,
    /// Smart-cut preset for long strips
    #[arg(long, value_enum, default_value_t = PresetArg::Balanced)]
    pub cut_preset: PresetArg,
    #[arg(long, short)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub list_devices: bool,
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

pub fn run(args: Args) -> anyhow::Result<()> {
    if args.list_devices {
        for d in DEVICES {
            println!(
                "{:<22} {} ({}x{}, {} dpi)",
                d.id, d.name, d.width, d.height, d.dpi
            );
        }
        return Ok(());
    }
    let Some(dev) = device(&args.device) else {
        bail!("unknown device '{}' (try --list-devices)", args.device);
    };
    let _opts = Preset::from(args.cut_preset).options();
    bail!(
        "export for {} is not implemented yet ({} chapter dirs given)",
        dev.name,
        args.chapters.len()
    )
}
