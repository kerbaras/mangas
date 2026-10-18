//! `mangas export <id> --preset show|save|clear`: the series export preset, and the note an
//! export prints when it uses one.

use mangas_core::export::ExportPreset;
use mangas_core::library::{self, Library};

use crate::output::outln;

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum PresetAction {
    /// Print the saved preset
    Show,
    /// Save the cut, format and profile flags given with it as the preset
    Save,
    /// Forget the preset; exports go back to the defaults
    Clear,
}

pub fn manage(
    lib: &Library,
    source: &str,
    manga_id: &str,
    action: PresetAction,
    explicit: &ExportPreset,
) -> anyhow::Result<()> {
    match action {
        PresetAction::Show => match library::get_export_preset(lib, source, manga_id)? {
            Some(p) => outln!("{}", serde_json::to_string_pretty(&p)?)?,
            None => outln!("no export preset for {source}/{manga_id}: exports use the defaults")?,
        },
        PresetAction::Save if explicit.is_empty() => anyhow::bail!(
            "nothing to save: give the options with it, e.g. --preset save --page-height 0.8 --trim-margins"
        ),
        PresetAction::Save => {
            library::set_export_preset(lib, source, manga_id, explicit)?;
            outln!("saved the export preset for {source}/{manga_id}")?;
        }
        PresetAction::Clear => {
            let had = library::clear_export_preset(lib, source, manga_id)?;
            outln!(
                "{}",
                if had {
                    "cleared the export preset"
                } else {
                    "there was no export preset"
                }
            )?;
        }
    }
    Ok(())
}

/// Tells the user a saved preset shapes this export (stderr, like progress).
pub fn note_series_preset(series: Option<&ExportPreset>) {
    if series.is_some() {
        eprintln!("using the series export preset (see `mangas export <id> --preset show`)");
    }
}
