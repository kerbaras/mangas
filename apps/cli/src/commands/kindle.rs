use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context};
use clap::Args as ClapArgs;
use mangas_core::export::kindle::{device, export, ExportRequest, Progress, DEVICES};
use mangas_core::library;
use mangas_core::ml;
use mangas_core::smartcut::{Options, Preset};

use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Directories of downloaded chapters, in reading order
    pub chapters: Vec<PathBuf>,
    /// Target device id (see --list-devices)
    #[arg(long, default_value = "kindle-paperwhite12")]
    pub device: String,
    /// Smart-cut preset for long strips
    #[arg(long, value_enum, default_value_t = PresetArg::Balanced)]
    pub cut_preset: PresetArg,
    /// Override the preset's max shrink (1.0 = never shrink, 1.25 = up to 25% more rows per page)
    #[arg(long)]
    pub max_shrink: Option<f64>,
    /// Override the preset's overlap after a forced cut (fraction of a page, e.g. 0.075)
    #[arg(long)]
    pub overlap: Option<f64>,
    /// Run the ONNX bubble/text/face detectors (needs a build with the `ml` feature)
    #[arg(long)]
    pub detectors: bool,
    /// EPUB title (defaults to the first chapter directory's parent name)
    #[arg(long)]
    pub title: Option<String>,
    /// Output EPUB path [default: out.epub]
    #[arg(long, short)]
    pub output: Option<PathBuf>,
    /// Print the known device profiles and exit
    #[arg(long)]
    pub list_devices: bool,
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum PresetArg {
    Balanced,
    FewestCuts,
    NoShrink,
}

impl From<PresetArg> for Preset {
    fn from(p: PresetArg) -> Self {
        match p {
            PresetArg::Balanced => Preset::Balanced,
            PresetArg::FewestCuts => Preset::FewestCuts,
            PresetArg::NoShrink => Preset::NoShrink,
        }
    }
}

fn list_devices() -> anyhow::Result<()> {
    for d in DEVICES {
        outln!(
            "{:<22} {} ({}x{}, {} dpi)",
            d.id,
            d.name,
            d.width,
            d.height,
            d.dpi
        )?;
    }
    Ok(())
}

fn cut_options(args: &Args) -> Options {
    let mut opts = Preset::from(args.cut_preset).options();
    if let Some(s) = args.max_shrink {
        opts.max_shrink = s;
    }
    if let Some(o) = args.overlap {
        opts.overlap = o;
    }
    opts
}

fn default_title(chapters: &[PathBuf]) -> String {
    chapters
        .first()
        .and_then(|c| c.canonicalize().ok())
        .and_then(|c| {
            c.parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "Manga".to_string())
}

fn print_progress(p: Progress) {
    match p {
        Progress::Chapter { index, total, path } => {
            eprintln!("[{}/{}] {}", index + 1, total, path.display())
        }
        Progress::Pages { done } => eprint!("\r  pages: {done}"),
        Progress::Writing { pages } => eprintln!("\nwriting {pages} pages"),
    }
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    if args.list_devices {
        return list_devices();
    }
    if args.chapters.is_empty() {
        bail!("no chapter directories given");
    }
    let Some(dev) = device(&args.device) else {
        bail!("unknown device '{}' (try --list-devices)", args.device);
    };
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from("out.epub"));
    let penalizer = if args.detectors {
        let cache = paths::models()?;
        let mut on_progress = |id: &str, done: u64, total: Option<u64>| {
            eprint!(
                "\rdownloading {id}: {done}/{}",
                total.map(|t| t.to_string()).unwrap_or_default()
            );
        };
        let p = ml::row_penalizer(&cache, &mut on_progress)
            .await
            .context("detectors")?;
        eprintln!("detectors: {:?}", ml::status(&cache).execution_provider);
        Some(p)
    } else {
        None
    };
    let req = ExportRequest {
        chapter_dirs: args.chapters.clone(),
        device: dev,
        cut: cut_options(&args),
        title: args
            .title
            .clone()
            .unwrap_or_else(|| default_title(&args.chapters)),
        output: output.clone(),
    };
    let started = Instant::now();
    let stats = export(&req, penalizer, &mut print_progress)?;
    outln!(
        "{}: {} pages from {} chapters ({} strips, {} source rows); pagination {} ms, filters {} ms, total {} ms",
        stats.output.display(),
        stats.pages,
        req.chapter_dirs.len(),
        stats.strips,
        stats.source_rows,
        stats.pagination_ms,
        stats.filters_ms,
        started.elapsed().as_millis()
    )?;
    if stats.skipped > 0 {
        outln!(
            "skipped {} unsupported file(s), listed in the warnings above",
            stats.skipped
        )?;
    }
    mark_exported(&stats.exported_chapters(&req.chapter_dirs));
    Ok(())
}

/// Library chapters among `dirs` count as read from now on; folders from elsewhere are ignored.
/// Each folder is matched as made absolute and as resolved (symlinks), since the library may
/// store either spelling. The EPUB is already written, so a failed update only gets a warning.
fn mark_exported(dirs: &[&Path]) {
    let mut spellings: Vec<PathBuf> = dirs
        .iter()
        .flat_map(|d| [std::path::absolute(d).ok(), d.canonicalize().ok()])
        .flatten()
        .collect();
    spellings.sort();
    spellings.dedup();
    let dirs: Vec<&Path> = spellings.iter().map(PathBuf::as_path).collect();
    let marked = paths::library_db()
        .and_then(|db| Ok(library::open(&db)?))
        .and_then(|lib| Ok(library::mark_exported(&lib, &dirs)?));
    if let Err(e) = marked {
        eprintln!("warning: could not mark the chapters as exported: {e}");
    }
}
