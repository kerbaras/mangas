use std::path::PathBuf;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::domain::{Chapter, Manga};
use mangas_core::download;
use mangas_core::export::kindle::export;
use mangas_core::export::volume::{self, find_volume, plan, Grouping, VolumePlan};
use mangas_core::export::{ExportPreset, Format};
use mangas_core::library;

use super::cut_args::{print_progress, CutArgs, DEFAULT_PROFILE};
use super::deliver::{try_deliver, unsent, SendArg};
use super::export_args::FormatArg;
use super::export_cover::cover_file;
use super::export_preset::{self, PresetAction};
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

/// "1..12" is the same range as "1-12".
fn selection(spec: &str) -> String {
    spec.replace("..", "-")
}

fn plans(args: &Args, manga: &Manga, chapters: &[Chapter]) -> anyhow::Result<Vec<VolumePlan>> {
    let (chapters, grouping) = match (&args.chapters, args.every) {
        (Some(spec), _) => (
            download::select_chapters(chapters, &selection(spec))?,
            Grouping::Single,
        ),
        (None, Some(n)) => (chapters.to_vec(), Grouping::Every(n as usize)),
        (None, None) if args.by_volume || args.volume.is_some() => {
            (chapters.to_vec(), Grouping::ByVolume)
        }
        (None, None) => (chapters.to_vec(), Grouping::Single),
    };
    let all = plan(manga, &chapters, grouping, Some(&args.name), None);
    let picked = match &args.volume {
        Some(n) => match find_volume(&all, n) {
            Some(p) => vec![p.clone()],
            None => bail!("{}", missing_volume(n, &all)),
        },
        None => all,
    };
    Ok(match (&args.title, picked.as_slice()) {
        (Some(title), [one]) => vec![VolumePlan {
            title: title.clone(),
            file_name: volume::file_name(title),
            ..one.clone()
        }],
        (Some(_), _) => bail!(
            "--title names a single book; this export makes {}",
            picked.len()
        ),
        (None, _) => picked,
    })
}

fn missing_volume(n: &str, all: &[VolumePlan]) -> String {
    let known: Vec<&str> = all.iter().filter_map(|p| p.number.as_deref()).collect();
    if known.is_empty() {
        format!(
            "no downloaded chapter is in volume {n} (the source lists no volumes; try --chapters)"
        )
    } else {
        format!(
            "no downloaded chapter is in volume {n} (downloaded volumes: {})",
            known.join(", ")
        )
    }
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
    for p in &plans {
        let (first, last) = (
            &p.chapters[0].number,
            &p.chapters[p.chapters.len() - 1].number,
        );
        outln!(
            "{}  ({} chapters, {first}–{last})",
            args.output_dir
                .join(&p.file_name)
                .with_extension(format.id())
                .display(),
            p.chapters.len()
        )?;
    }
    if args.dry_run {
        return Ok(());
    }
    export_preset::note_series_preset(series.as_ref());
    let penalizer = args.cut.penalizer().await?;
    let cover = cover_file(&src.manga).await;
    let mut failed = 0;
    for p in &plans {
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
        let stats = export(&req, penalizer.clone(), &mut print_progress)?;
        outln!("{}: {} pages", stats.output.display(), stats.pages)?;
        if stats.skipped > 0 {
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
            if !try_deliver(&lib, &file, &req.title, via, kindle).await {
                failed += 1;
            }
        }
    }
    unsent(failed)
}
