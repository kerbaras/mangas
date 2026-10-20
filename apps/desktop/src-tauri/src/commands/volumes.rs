//! Volume export: plan the books of a manga's downloaded chapters, then write each one.

use std::path::PathBuf;

use mangas_core::export::kindle::{self, Progress};
use mangas_core::export::volume::{self, Grouping, VolumeOutcome, VolumePlan};
use mangas_core::export::{ExportPreset, Format};
use mangas_core::library;
use mangas_core::smartcut::Preset;
use tauri::{AppHandle, Runtime};

use super::covers::cover_file;
use super::export::{emit, hold_for_export, record_export, resolve, ExportEvent};
use super::penalizer::penalizer;
use super::{blocking, open_library, settings_or_default, settings_quietly, to_ipc};
use crate::paths;

fn plans(
    source: &str,
    manga_id: &str,
    chapter_ids: &[String],
    grouping: Grouping,
    title: Option<&str>,
) -> Result<(volume::VolumeSource, Vec<VolumePlan>), String> {
    let lib = open_library()?;
    let src = volume::load(&lib, source, manga_id, Some(chapter_ids)).map_err(to_ipc)?;
    let plans = volume::plan(&src.manga, &src.chapters, grouping, None, title);
    Ok((src, plans))
}

/// The files an export would write, in order; nothing is read from disk but the library.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub fn plan_volumes(
    source: String,
    manga_id: String,
    chapter_ids: Vec<String>,
    grouping: Grouping,
    title: Option<String>,
) -> Result<Vec<VolumePlan>, String> {
    Ok(plans(&source, &manga_id, &chapter_ids, grouping, title.as_deref())?.1)
}

/// The names `names` would be written under in `dir` now (` (2)`, ` (3)`… for taken ones), so
/// the export dialog previews the files the export really writes.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn free_export_names(dir: PathBuf, names: Vec<String>) -> Result<Vec<String>, String> {
    // Lists the folder on every keystroke, which can be slow on a network drive.
    blocking(move || {
        let dir =
            mangas_core::export::in_export_folder(&paths::exports(&settings_quietly())?, &dir);
        mangas_core::export::free_names_in(&dir, &names).map_err(to_ipc)
    })
    .await
}

/// The file name a book titled `title` is written under as `format` (shortened when too long),
/// for books planned in the UI (chapters of several series joined into one).
#[tauri::command]
pub fn export_file_name(title: String, format: Format) -> String {
    volume::file_name_as(&title, format)
}

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
#[allow(clippy::too_many_arguments)]
/// A relative `output_dir`, or none, goes under the default export folder ([`paths::exports`]).
pub async fn export_volumes<R: Runtime>(
    app: AppHandle<R>,
    source: String,
    manga_id: String,
    chapter_ids: Vec<String>,
    grouping: Grouping,
    title: Option<String>,
    device: Option<String>,
    format: Option<Format>,
    preset: Option<Preset>,
    detectors: bool,
    output_dir: Option<PathBuf>,
    replace: Option<bool>,
    quality: Option<u8>,
    split_mb: Option<u64>,
) -> Result<VolumeOutcome, String> {
    // Before the chapter folders are looked up: a data-folder move can't start under the export.
    let _reading = hold_for_export().map_err(to_ipc)?;
    let settings = settings_or_default();
    let output_dir = mangas_core::export::in_export_folder(
        &paths::exports(&settings)?,
        &output_dir.unwrap_or_default(),
    );
    let explicit = ExportPreset {
        cut_preset: preset,
        format,
        profile: device,
        jpeg_quality: quality,
        ..Default::default()
    };
    let series =
        library::get_export_preset(&open_library()?, &source, &manga_id).map_err(to_ipc)?;
    let resolved = resolve(&explicit, series.as_ref(), &settings);
    let dev = resolved
        .profile_or(paths::DEFAULT_PROFILE)
        .map_err(to_ipc)?;
    let device = dev.id.to_string();
    let format = Format::resolve(None, None, resolved.format, &dev).map_err(to_ipc)?;
    let (src, plans) = plans(&source, &manga_id, &chapter_ids, grouping, title.as_deref())?;
    let replace = replace.unwrap_or(false);
    let plans = volume::one_file_each(plans, replace);
    let plans =
        volume::named_as_written(plans, &output_dir, format.id(), replace).map_err(to_ipc)?;
    if plans.is_empty() {
        return Err(to_ipc("none of these chapters is downloaded"));
    }
    let detection = penalizer(&app, detectors, &settings).await?;
    let cover = cover_file(&src.manga).await;
    let reqs: Vec<_> = plans
        .iter()
        .map(|plan| {
            volume::request(
                &src.manga,
                plan,
                dev,
                resolved.cut(),
                &output_dir,
                cover.clone(),
            )
            .with_format(format)
            .with_page(resolved.page())
            .with_replace(replace)
            .with_split(split_mb.map(|mb| mb * 1_000_000))
        })
        .collect();
    let detectors_error = detection.error.clone();
    let mut outcome = tauri::async_runtime::spawn_blocking(move || {
        let total = reqs.len();
        volume::export_each(&reqs, |index, req| {
            let title = req.title.clone();
            emit(
                &app,
                ExportEvent::Volume {
                    index,
                    total,
                    title,
                },
            );
            let mut on_progress = |p: Progress| emit(&app, ExportEvent::Export(p));
            let stats = kindle::export(req, detection.penalizer.clone(), &mut on_progress)?;
            record_export(&stats, &req.chapter_dirs, &req.title, &device);
            Ok(stats)
        })
    })
    .await
    .map_err(to_ipc)?;
    for stats in &mut outcome.written {
        stats.detectors_error = detectors_error.clone();
    }
    Ok(outcome)
}
