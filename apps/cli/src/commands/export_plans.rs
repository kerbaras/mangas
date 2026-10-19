//! Which books `mangas export` makes: the chapters picked and how they are grouped.

use anyhow::bail;
use mangas_core::domain::{Chapter, Manga};
use mangas_core::download;
use mangas_core::export::volume::{self, find_volume, plan, Grouping, VolumePlan};

use super::export::Args;

/// "1..12" is the same range as "1-12".
fn selection(spec: &str) -> String {
    spec.replace("..", "-")
}

pub fn plans(args: &Args, manga: &Manga, chapters: &[Chapter]) -> anyhow::Result<Vec<VolumePlan>> {
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
