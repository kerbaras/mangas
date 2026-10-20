//! The export dialog's size estimate, so it can warn about books over the Send-to-Kindle limit
//! before writing them.

use std::path::PathBuf;

use mangas_core::export::kindle::{self, Estimate};
use mangas_core::export::ExportPreset;
use mangas_core::smartcut::Preset;

use super::export::{resolve, series_preset};
use super::{blocking, settings_quietly, to_ipc};
use crate::paths;

/// The estimated pages and bytes of the largest of `books` (each its chapter folders) with the
/// dialog's choices over the series preset, as `export_volumes` resolves them.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn estimate_export(
    books: Vec<Vec<PathBuf>>,
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
        let dirs: Vec<PathBuf> = books.iter().flatten().cloned().collect();
        let resolved = resolve(
            &explicit,
            series_preset(&dirs).as_ref(),
            &settings_quietly(),
        );
        let dev = resolved
            .profile_or(paths::DEFAULT_PROFILE)
            .map_err(to_ipc)?;
        kindle::estimate_largest(&books, dev, resolved.cut(), resolved.page()).map_err(to_ipc)
    })
    .await
}
