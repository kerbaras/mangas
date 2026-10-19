//! The export dialog's size estimate, so it can warn about books over the Send-to-Kindle limit
//! before writing them.

use std::path::PathBuf;

use mangas_core::export::kindle::{self, Estimate};
use mangas_core::export::ExportPreset;
use mangas_core::smartcut::Preset;

use super::export::resolve;
use super::{blocking, settings_quietly, to_ipc};
use crate::paths;

/// The estimated pages and bytes of one book of `chapter_dirs` with the dialog's choices.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn estimate_export(
    chapter_dirs: Vec<PathBuf>,
    device: Option<String>,
    preset: Option<Preset>,
    quality: Option<u8>,
) -> Result<Estimate, String> {
    blocking(move || {
        let explicit = ExportPreset {
            cut_preset: preset,
            profile: device,
            jpeg_quality: quality,
            ..Default::default()
        };
        let resolved = resolve(&explicit, None, &settings_quietly());
        let dev = resolved
            .profile_or(paths::DEFAULT_PROFILE)
            .map_err(to_ipc)?;
        kindle::estimate(&chapter_dirs, dev, resolved.cut(), resolved.page()).map_err(to_ipc)
    })
    .await
}
