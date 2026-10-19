use std::path::PathBuf;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::export::kindle::export;
use mangas_core::export::volume;
use mangas_core::export::{ExportPreset, Format};
use mangas_core::library;

use super::cut_args::{print_progress, CutArgs, DEFAULT_PROFILE};
use super::deliver::{try_deliver, unsent, SendArg};
use super::export_args::FormatArg;
use super::export_cover::cover_file;
use super::export_plans::plans;
use super::export_preset::{self, PresetAction};
use super::export_report::Report;
use super::kindle_record;
use super::{hint, stored_settings};
use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
#[command(group = clap::ArgGroup::new("bundle").args(["volume", "chapters", "by_volume", "every"]))]
pub struct Args {
    /// Manga id on the source (see `mangas list`)
    pub manga_id: String,
    #[arg(long, default_value = "mangadex")]
    pub source: String,
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
    /// Print the files that would be written and exit
    #[arg(long)]
    pub dry_run: bool,
    /// Print one JSON object instead of the lines above: `files` (path, title, chapters, pages,
    /// skipped_files), `skipped` and `failed` (path, reason) and `detectors` (state "used",
    /// "skipped" or "off", and the runtime's error as reason)
    #[arg(long, conflicts_with_all = ["dry_run", "preset"])]
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
    #[command(flatten)]
    pub cut: CutArgs,
    /// Manage the series export preset instead of exporting. Exports of the series use it for
    /// every option not given on the command line; `save` stores the cut, format and profile
    /// flags given with it
    #[arg(long, value_enum, value_name = "ACTION")]
    pub preset: Option<PresetAction>,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let explicit = args.cut.explicit(args.format.map(Format::from))?;
    if let Some(action) = args.preset {
        return export_preset::manage(&lib, &args.source, &args.manga_id, action, &explicit);
    }
    let src = volume::load(&lib, &args.source, &args.manga_id, None)?;
    let plans = volume::one_file_each(plans(&args, &src.manga, &src.chapters)?, args.replace);
    if plans.is_empty() {
        let list = hint(&format!("chapters {}", args.manga_id), &args.source);
        bail!("nothing to export: no downloaded chapters match (see {list})");
    }
    let series = library::get_export_preset(&lib, &args.source, &args.manga_id)?;
    let fallback = ExportPreset::from_settings(&stored_settings());
    let preset = ExportPreset::resolve(&explicit, series.as_ref(), &fallback);
    let device = CutArgs::device_of(&preset, DEFAULT_PROFILE)?;
    let format = Format::resolve(preset.format, None, None, &device)?;
    let names = volume::written_names(&plans, &args.output_dir, format.id(), args.replace)?;
    for (p, name) in plans.iter().zip(&names).filter(|_| !args.json) {
        let (first, last) = (
            &p.chapters[0].number,
            &p.chapters[p.chapters.len() - 1].number,
        );
        outln!(
            "{}  ({} chapters, {first}–{last})",
            args.output_dir.join(name).display(),
            p.chapters.len()
        )?;
    }
    if args.dry_run {
        return Ok(());
    }
    export_preset::note_series_preset(series.as_ref());
    let (penalizer, detectors) = args.cut.penalizer().await?;
    let mut report = Report::new(detectors);
    let cover = cover_file(&src.manga).await;
    for (p, name) in plans.iter().zip(&names) {
        let req = volume::request(
            &src.manga,
            p,
            device,
            preset.cut(),
            &args.output_dir,
            cover.clone(),
        )
        .with_format(format)
        .with_page(preset.page())
        .with_replace(args.replace);
        let stats = match export(&req, penalizer.clone(), &mut print_progress) {
            Ok(stats) => stats,
            Err(e) if args.json => {
                report.fail(&args.output_dir.join(name), e.to_string());
                report.print()?;
                return Err(e.into());
            }
            Err(e) => return Err(e.into()),
        };
        report.add(&stats, &req.title, &req.chapter_dirs);
        if !args.json {
            outln!("{}: {} pages", stats.output.display(), stats.pages)?;
        }
        if stats.skipped > 0 && !args.json {
            outln!(
                "skipped {} unsupported file(s), listed in the warnings above",
                stats.skipped
            )?;
        }
        kindle_record::mark_exported(&lib, &stats.exported_chapters(&req.chapter_dirs));
        kindle_record::record_export(&lib, &stats.output, &req.title, device.id);
        if let Some(via) = args.send {
            let file = std::path::absolute(&stats.output)?;
            let kindle = args.kindle.as_deref();
            match try_deliver(&lib, &file, &req.title, via, kindle).await {
                Ok(sent) if args.json => eprintln!("{sent}"),
                Ok(sent) => outln!("{sent}")?,
                Err(e) => report.fail(&stats.output, e),
            }
        }
    }
    if args.json {
        report.print()?;
    }
    unsent(report.failed.len())
}
