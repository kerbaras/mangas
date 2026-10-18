use std::path::PathBuf;

use anyhow::bail;
use clap::Args as ClapArgs;
use mangas_core::covers::{self, CoverCache, Limits};
use mangas_core::domain::{Chapter, Manga};
use mangas_core::download;
use mangas_core::export::kindle::export;
use mangas_core::export::volume::{self, find_volume, plan, Grouping, VolumePlan};
use mangas_core::export::Format;
use mangas_core::library;
use mangas_providers::covers::CoverFetcher;

use super::cut_args::{print_progress, CutArgs, DEFAULT_PROFILE};
use super::export_args::FormatArg;
use super::kindle::mark_exported;
use super::{source_by_name, stored_settings};
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
    #[command(flatten)]
    pub cut: CutArgs,
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

/// The cached series cover, fetched once if needed; `None` (first page as cover) when offline.
async fn cover_file(manga: &Manga) -> Option<PathBuf> {
    let url = manga.cover_url.as_deref()?;
    let cache = CoverCache::new(paths::folders().ok()?.covers, Limits::default());
    let fetcher = source_by_name(&manga.source).and_then(|s| Ok(CoverFetcher::for_source(s)?));
    let fetched = match fetcher {
        Ok(f) => covers::cover(&cache, &f, &manga.source, &manga.id.0, url)
            .await
            .map_err(anyhow::Error::from),
        Err(e) => Err(e),
    };
    fetched
        .inspect_err(|e| eprintln!("warning: no series cover ({e}); the first page is the cover"))
        .ok()
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let lib = library::open(&paths::library_db()?)?;
    let src = volume::load(&lib, &args.source, &args.manga_id, None)?;
    let plans = plans(&args, &src.manga, &src.chapters)?;
    if plans.is_empty() {
        bail!(
            "nothing to export: no downloaded chapters match (see `mangas chapters {}`)",
            args.manga_id
        );
    }
    let stored = stored_settings();
    let saved = stored.default_device.as_deref().unwrap_or(DEFAULT_PROFILE);
    let device = args.cut.device_or(saved)?;
    let explicit = args.format.map(Format::from);
    let format = Format::resolve(explicit, None, stored.default_format, &device)?;
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
    let penalizer = args.cut.penalizer().await?;
    let cover = cover_file(&src.manga).await;
    for p in &plans {
        let req = volume::request(
            &src.manga,
            p,
            device,
            args.cut.options(),
            &args.output_dir,
            cover.clone(),
        )
        .with_format(format);
        let stats = export(&req, penalizer.clone(), &mut print_progress)?;
        outln!("{}: {} pages", stats.output.display(), stats.pages)?;
        if stats.skipped > 0 {
            outln!(
                "skipped {} unsupported file(s), listed in the warnings above",
                stats.skipped
            )?;
        }
        mark_exported(&stats.exported_chapters(&req.chapter_dirs));
    }
    Ok(())
}
