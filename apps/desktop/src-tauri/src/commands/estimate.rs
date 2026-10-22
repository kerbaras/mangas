//! The export dialog's size plan: each book's estimate against the budget of the delivery, so it
//! can show which books are over the Send-to-Kindle limit before writing them, and the levers
//! for a book written over it.

use std::path::PathBuf;

use mangas_core::export::kindle::{self, BookBudget, BudgetSpec, Delivery, Levers};
use mangas_core::export::{ExportPreset, Format};
use mangas_core::smartcut::Preset;

use super::export::{resolve, series_preset};
use super::{blocking, settings_quietly, to_ipc};
use crate::paths;

/// Each of `books` (its chapter folders) sized with the dialog's choices over the series preset,
/// as `export_volumes` resolves them, against 50 MB when `email`, else 200 MB.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn estimate_export(
    books: Vec<Vec<PathBuf>>,
    device: Option<String>,
    preset: Option<Preset>,
    quality: Option<u8>,
    email: bool,
    split_mb: Option<u64>,
    format: Option<Format>,
) -> Result<Vec<BookBudget>, String> {
    blocking(move || {
        let spec = spec_of(&books, device, preset, quality, email, split_mb, format)?;
        kindle::plan_budget(&books, spec).map_err(to_ipc)
    })
    .await
}

/// What makes each of `books` fit, as its plan offers it (`kindle::book_levers`): for books
/// written over their budget, which the plan may have estimated under it.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn fit_levers(
    books: Vec<Vec<PathBuf>>,
    device: Option<String>,
    preset: Option<Preset>,
    quality: Option<u8>,
    email: bool,
    split_mb: Option<u64>,
    format: Option<Format>,
) -> Result<Vec<Levers>, String> {
    blocking(move || {
        let spec = spec_of(&books, device, preset, quality, email, split_mb, format)?;
        (books.iter())
            .map(|b| kindle::book_levers(b, spec).map_err(to_ipc))
            .collect()
    })
    .await
}

/// The budget spec of the dialog's choices over the series preset of `books`.
#[allow(clippy::too_many_arguments)]
fn spec_of(
    books: &[Vec<PathBuf>],
    device: Option<String>,
    preset: Option<Preset>,
    quality: Option<u8>,
    email: bool,
    split_mb: Option<u64>,
    format: Option<Format>,
) -> Result<BudgetSpec, String> {
    let explicit = ExportPreset {
        cut_preset: preset,
        format,
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
    Ok(BudgetSpec {
        device: dev,
        cut: resolved.cut(),
        page: resolved.page(),
        delivery: if email {
            Delivery::Email
        } else {
            Delivery::Other
        },
        format: Format::resolve(None, None, resolved.format, &dev).map_err(to_ipc)?,
        split: split_mb.map(|mb| mb * 1_000_000),
    })
}
