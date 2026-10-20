//! `mangas settings quality`: the JPEG quality exports use for a device profile, shared with the
//! app's export dialog and Settings → Export.

use anyhow::{anyhow, bail};
use clap::Args as ClapArgs;
use mangas_core::export::{default_quality, profile, JPEG_QUALITY};
use mangas_core::library::Settings;

use crate::output::outln;

#[derive(ClapArgs)]
pub struct QualityArgs {
    /// Device profile id (see `mangas kindle --list-profiles`); lists the stored ones if omitted
    profile: Option<String>,
    /// 40-95 (lower makes smaller books), or "default" for the profile's own
    value: Option<String>,
}

pub fn run(args: QualityArgs) -> anyhow::Result<()> {
    let Some(id) = args.profile else {
        let stored = super::stored_settings().jpeg_quality;
        if stored.is_empty() {
            outln!("every profile uses its own (90 at 300 dpi, else 85)")?;
        }
        for (id, q) in stored {
            outln!("{id:<24}{q}")?;
        }
        return Ok(());
    };
    let device = profile(&id)
        .ok_or_else(|| anyhow!("unknown profile '{id}' (see mangas kindle --list-profiles)"))?;
    let settings = match args.value.as_deref() {
        None => super::stored_settings(),
        Some(raw) => {
            let quality = parse(raw)?;
            super::settings::save(|s| with_quality(s, &id, quality))?
        }
    };
    match settings.jpeg_quality.get(&id) {
        Some(q) => outln!("{q}")?,
        None => outln!("{} (the profile's own)", default_quality(device.dpi))?,
    }
    Ok(())
}

fn parse(raw: &str) -> anyhow::Result<Option<u8>> {
    let (lo, hi) = JPEG_QUALITY;
    match raw {
        "default" => Ok(None),
        _ => match raw.parse::<u8>() {
            Ok(q) if (lo..=hi).contains(&q) => Ok(Some(q)),
            _ => bail!("JPEG quality must be {lo}-{hi} or \"default\", not '{raw}'"),
        },
    }
}

fn with_quality(s: &Settings, id: &str, quality: Option<u8>) -> Settings {
    let mut out = s.clone();
    match quality {
        Some(q) => out.jpeg_quality.insert(id.to_string(), q),
        None => out.jpeg_quality.remove(id),
    };
    out
}
