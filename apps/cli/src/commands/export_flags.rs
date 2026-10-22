//! The flags of `mangas export`.

use std::path::PathBuf;

use clap::Args as ClapArgs;
use mangas_core::export::volume;

use super::cut_args::CutArgs;
use super::deliver::SendArg;
use super::export_args::FormatArg;
use super::export_preset::PresetAction;

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
    /// File format [default: for a Kindle, AZW3 with `--send usb` and EPUB with `--send email`;
    /// else the stored default, else what the profile recommends]
    #[arg(long, value_enum)]
    pub format: Option<FormatArg>,
    /// Folder the files are written to
    #[arg(long, short, default_value = ".")]
    pub output_dir: PathBuf,
    /// Print the plan and exit: each file that would be written with its estimated size
    /// against the budget (50 MB with `--send email`, else 200 MB) and the JPEG quality
    #[arg(long)]
    pub dry_run: bool,
    /// Print one JSON object instead of the lines above: `plan` (path, title, chapters, format,
    /// format_reason when a Kindle route picked it, pages, estimated_bytes, budget_bytes,
    /// over_budget, jpeg_quality, files, levers when over; the
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
    /// With `--send usb`, copy a file that is no export format (the Kindle may not open it);
    /// EPUB and CBZ are never copied
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
