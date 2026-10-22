use anyhow::bail;
use mangas_core::domain::MangaId;
use mangas_core::export::kindle::check_export_folder;
use mangas_core::export::volume::{self, VolumeOutcome};
use mangas_core::export::{for_delivery, ExportPreset, Format};
use mangas_core::{library, storage};

use super::cut_args::{CutArgs, DEFAULT_PROFILE};
use super::deliver::{try_deliver, unsent};
use super::deliver_precheck;
use super::export_cover::cover_file;
use super::export_plan;
use super::export_plan::Fewer;
use super::export_plans::plans;
use super::export_preset;
use super::export_report::Report;
use super::export_size as size;
use super::followed::followed_source;
use super::kindle_record;
use super::{export_progress::export_printing, hint, stored_settings};
use crate::output::outln;
use crate::paths;

pub use super::export_flags::Args;

pub async fn run(args: Args) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let explicit = args.cut.explicit(args.format.map(Format::from))?;
    let id = MangaId(args.manga_id.clone());
    let source = followed_source(&lib, &id, args.source.as_deref())?;
    if let Some(action) = args.preset {
        return export_preset::manage(&lib, &source, &args.manga_id, action, &explicit);
    }
    // Before the chapter folders are looked up: a data-folder move can't start under the export.
    let _reading = storage::hold_for_export(paths::data_dir)?;
    let src = volume::load(&lib, &source, &args.manga_id, None)?;
    let plans = volume::one_file_each(plans(&args, &src.manga, &src.chapters)?, args.replace);
    if plans.is_empty() {
        let list = hint(&format!("chapters {}", args.manga_id), &source);
        bail!("nothing to export: no downloaded chapters match (see {list})");
    }
    let series = library::get_export_preset(&lib, &source, &args.manga_id)?;
    let stored = stored_settings();
    let fallback = ExportPreset::from_settings(&stored);
    let preset = ExportPreset::resolve(&explicit, series.as_ref(), &fallback)
        .with_stored_quality(&stored, DEFAULT_PROFILE);
    let device = CutArgs::device_of(&preset, DEFAULT_PROFILE)?;
    let route = size::route_of(args.send);
    let chosen = for_delivery(explicit.format, preset.format, &device, route);
    let format = chosen.format;
    check_export_folder(&args.output_dir, args.cut.split_mb.is_some(), args.replace)?;
    let plans = volume::named_as_written(plans, &args.output_dir, format.id(), args.replace)?;
    let names: Vec<String> = plans.iter().map(|p| p.file_name.clone()).collect();
    let firsts = volume::first_part_names(&names, &args.output_dir, args.replace)?;
    let spec = size::budget_spec(&preset, device, args.send, format, args.cut.split());
    let plan = export_plan::size(&plans, &names, &args.output_dir, spec, chosen)?;
    let fewer = Fewer::of(args.chapters.is_some() || args.volume.is_some());
    match (args.json, args.dry_run) {
        (true, true) => return export_plan::print_json(&plan),
        (false, _) => export_plan::print(&plans, &plan, &firsts, args.cut.split_mb, fewer)?,
        (true, false) => {}
    }
    if args.dry_run {
        return Ok(());
    }
    let books: Vec<_> = names.iter().map(|n| args.output_dir.join(n)).collect();
    let kindle = args.kindle.as_deref();
    let to = args.send.map(|via| (via, kindle, args.force));
    deliver_precheck::precheck(to, format.id(), &books, args.json)?;
    export_preset::note_series_preset(series.as_ref());
    let (penalizer, detectors) = args.cut.penalizer().await?;
    let mut report = Report::new(detectors).with_plan(plan);
    let cover = cover_file(&src.manga).await;
    let reqs: Vec<_> = plans
        .iter()
        .map(|p| {
            volume::request(
                &src.manga,
                p,
                device,
                preset.cut(),
                &args.output_dir,
                cover.clone(),
            )
            .with_format(format)
            .with_page(preset.page())
            .with_direction(preset.direction)
            .with_replace(args.replace)
            .with_split(args.cut.split())
        })
        .collect();
    let mut written = Vec::with_capacity(reqs.len());
    for (index, req) in reqs.iter().enumerate() {
        let stats = match export_printing(req, penalizer.clone()) {
            Ok(stats) => stats,
            Err(e) => {
                let outcome = VolumeOutcome::stopped(&reqs, index, written, e.to_string());
                report.stopped(&outcome);
                if args.json {
                    report.print()?;
                }
                bail!(outcome.summary().unwrap_or_else(|| e.to_string()));
            }
        };
        report.add(&stats, &req.title, &req.chapter_dirs);
        if !args.json {
            size::print_written(&stats)?;
        }
        let planned = report
            .plan
            .get(index)
            .and_then(|b| b.budget.levers.as_ref());
        let fix = || size::levers_text(planned, &req.chapter_dirs, spec, fewer);
        size::warn_too_big(&stats.oversized, size::delivery_of(args.send), fix);
        let recorded = kindle_record::mark_exported(&lib, &stats, &req.chapter_dirs);
        report.recorded.extend(recorded);
        for (output, title) in &stats.titled_outputs(&req.title) {
            kindle_record::record_export(&lib, output, title, device.id);
            let Some(via) = args.send else { continue };
            match try_deliver(&lib, output, title, via, kindle, args.force).await {
                Ok(sent) if args.json => eprintln!("{sent}"),
                Ok(sent) => outln!("{sent}")?,
                Err(e) => report.unsent(output, &e),
            }
        }
        written.push(stats);
    }
    if args.json {
        report.print()?;
    }
    unsent(report.failed.len())
}
