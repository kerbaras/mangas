use std::path::PathBuf;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::domain::MangaId;
use mangas_core::export::kindle::{check_export_folder, BudgetSpec};
use mangas_core::export::volume::{self, VolumeOutcome};
use mangas_core::export::{ExportPreset, Format};
use mangas_core::{library, storage};

use super::cut_args::{CutArgs, DEFAULT_PROFILE};
use super::deliver::{try_deliver, unsent, SendArg};
use super::deliver_usb;
use super::export_args::FormatArg;
use super::export_cover::cover_file;
use super::export_plan;
use super::export_plan::Fewer;
use super::export_plans::plans;
use super::export_preset::{self, PresetAction};
use super::export_report::Report;
use super::export_size::{delivery_of, levers_text, print_written, warn_too_big};
use super::followed::followed_source;
use super::kindle_record;
use super::{export_progress::export_printing, hint, stored_settings};
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
#[command(group = clap::ArgGroup::new("bundle").args(["volume", "chapters", "by_volume", "every"]))]
pub struct Args {
    /// Manga id on the source (see `mangas list`)
    pub manga_id: String,
    /// Source of the title [default: the one your library follows it on]
    #[arg(long)]
    pub source: Option<String>,
    /// Export one volume, as numbered by the provider
    #[arg(long)]
    pub volume: Option<String>,
    /// Chapters for one book, e.g. "1..12" or "1-5,7" [default: every downloaded chapter]
    #[arg(long)]
    pub chapters: Option<String>,
    /// One book per provider volume; chapters without a volume share the last one
    #[arg(long)]
    pub by_volume: bool,
    /// One book per this many chapters
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
    pub every: Option<u32>,
    /// Title of a single book, also its file name
    #[arg(long)]
    pub title: Option<String>,
    /// Title template: {series}, {volume}, {first}, {last}
    #[arg(long, default_value = volume::DEFAULT_TEMPLATE)]
    pub name: String,
    /// File format [default: the stored default, else what the profile recommends]
    #[arg(long, value_enum)]
    pub format: Option<FormatArg>,
    /// Folder the files are written to
    #[arg(long, short, default_value = ".")]
    pub output_dir: PathBuf,
    /// Print the plan and exit: each file that would be written with its estimated size
    /// against the budget (50 MB with `--send email`, else 200 MB) and the JPEG quality
    #[arg(long)]
    pub dry_run: bool,
    /// Print one JSON object instead of the lines above: `plan` (path, title, chapters, pages,
    /// estimated_bytes, budget_bytes, over_budget, jpeg_quality, files, levers when over; the
    /// only key with --dry-run), `files` (path, title, chapters, pages, bytes, part {index,
    /// total} when written in parts, skipped_files, jpeg_quality), `skipped`, `failed` and
    /// `not_written` (path, reason; a failed book stops the run: the books before it stay, the
    /// ones after it are `not_written`) and `detectors` (state "used", "skipped" or "off", and the
    /// runtime's error as reason)
    #[arg(long, conflicts_with = "preset")]
    pub json: bool,
    /// Write over books already in the folder instead of next to them as "Title (2)"
    #[arg(long)]
    pub replace: bool,
    /// After writing, email each book to the Kindle or copy it to one over USB
    #[arg(long, value_enum)]
    pub send: Option<SendArg>,
    /// Mount point of the Kindle for `--send usb` [default: detected]
    #[arg(long)]
    pub kindle: Option<PathBuf>,
    /// With `--send usb`, copy a format the Kindle may not open (EPUB); CBZ is never copied
    #[arg(long, requires = "send")]
    pub force: bool,
    #[command(flatten)]
    pub cut: CutArgs,
    /// Manage the series export preset instead of exporting. Exports of the series use it for
    /// every option not given on the command line; `save` stores the cut, format, profile and
    /// direction flags given with it
    #[arg(long, value_enum, value_name = "ACTION")]
    pub preset: Option<PresetAction>,
}

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
    let format = Format::resolve(preset.format, None, None, &device)?;
    check_export_folder(&args.output_dir, args.cut.split_mb.is_some(), args.replace)?;
    let plans = volume::named_as_written(plans, &args.output_dir, format.id(), args.replace)?;
    let names: Vec<String> = plans.iter().map(|p| p.file_name.clone()).collect();
    let firsts = volume::first_part_names(&names, &args.output_dir, args.replace)?;
    let delivery = delivery_of(args.send);
    let spec = BudgetSpec {
        device,
        cut: preset.cut(),
        page: preset.page(),
        delivery,
        split: args.cut.split(),
    };
    let plan = export_plan::size(&plans, &names, &args.output_dir, spec)?;
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
    deliver_usb::precheck(to, format.id(), &books, args.json)?;
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
            print_written(&stats)?;
        }
        let planned = report
            .plan
            .get(index)
            .and_then(|b| b.budget.levers.as_ref());
        let fix = || levers_text(planned, &req.chapter_dirs, spec, fewer);
        warn_too_big(&stats.oversized, delivery, fix);
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
