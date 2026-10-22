//! The Send-to-Kindle size check after a book is written: a file over its delivery's budget
//! (50 MB by email, 200 MB otherwise) is named on stderr with the levers its plan offers.

use std::path::{Path, PathBuf};

use mangas_core::export::kindle::{
    book_levers, BudgetSpec, Delivery, Device, ExportStats, Levers, Oversized,
};

use mangas_core::export::{ExportPreset, Format, Route};

use super::deliver::SendArg;
use super::export_plan::{levers_line, Fewer};

use crate::output::outln;

/// Size of `file` in bytes; 0 when it can't be read.
pub fn bytes(file: &Path) -> u64 {
    std::fs::metadata(file).map(|m| m.len()).unwrap_or(0)
}

/// The route `--send` takes, which picks a Kindle book's format (`export::for_delivery`).
pub fn route_of(send: Option<SendArg>) -> Route {
    match send {
        Some(SendArg::Email) => Route::Email,
        Some(SendArg::Usb) => Route::Usb,
        None => Route::Save,
    }
}

/// What the books are sized against: `send`'s delivery budget, with pages costed in `format`.
pub fn budget_spec(
    preset: &ExportPreset,
    device: Device,
    send: Option<SendArg>,
    format: Format,
    split: Option<u64>,
) -> BudgetSpec {
    let (cut, page, delivery) = (preset.cut(), preset.page(), delivery_of(send));
    BudgetSpec {
        device,
        cut,
        page,
        delivery,
        format,
        split,
    }
}

/// The budget `send` puts a book under: email's, or the web one for everything else.
pub fn delivery_of(send: Option<SendArg>) -> Delivery {
    match send {
        Some(SendArg::Email) => Delivery::Email,
        _ => Delivery::Other,
    }
}

/// Warns about each written file over `delivery`'s budget (measured on disk by core) with `fix`,
/// the plan's lever line, worked out only when a file is over.
pub fn warn_too_big(files: &[Oversized], delivery: Delivery, fix: impl FnOnce() -> Option<String>) {
    let over = delivery.over(files);
    if over.is_empty() {
        return;
    }
    let fix = fix().unwrap_or_else(|| {
        format!(
            "to fit: --split-mb {} (parts)",
            delivery.target() / 1_000_000
        )
    });
    for file in over {
        eprintln!("{}", too_big(&file.path, file.bytes, delivery, &fix));
    }
}

/// The plan's lever line for a written book of `dirs`: its `planned` levers, else core's, so the
/// warning never offers what the plan would not.
pub fn levers_text(
    planned: Option<&Levers>,
    dirs: &[PathBuf],
    spec: BudgetSpec,
    fewer: Fewer,
) -> Option<String> {
    let levers = match planned {
        Some(l) => l.clone(),
        None => book_levers(dirs, spec).ok()?,
    };
    Some(levers_line(&levers, fewer))
}

/// The warning for `file` of `size` bytes over `delivery`'s budget, with the lever line `fix`.
pub fn too_big(file: &Path, size: u64, delivery: Delivery, fix: &str) -> String {
    format!(
        "warning: {} is {:.1} MB, over the {} MB Send-to-Kindle {} limit; {fix}",
        file.display(),
        size as f64 / 1e6,
        delivery.budget() / 1_000_000,
        delivery.limit_name(),
    )
}

/// Each file written and its page count: the parts, else the one book.
pub fn written_pages(stats: &ExportStats) -> Vec<(PathBuf, usize)> {
    match stats.parts.is_empty() {
        true => vec![(stats.output.clone(), stats.pages)],
        false => stats
            .parts
            .iter()
            .map(|p| (p.path.clone(), p.pages))
            .collect(),
    }
}

/// The files a book was written to, with their pages, and the files it skipped.
pub fn print_written(stats: &ExportStats) -> std::io::Result<()> {
    for (file, pages) in written_pages(stats) {
        outln!("{}: {pages} pages", file.display())?;
    }
    if stats.skipped > 0 {
        outln!(
            "skipped {} unsupported file(s), listed in the warnings above",
            stats.skipped
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_warned_about_against_the_budget_of_its_delivery() {
        let file = Path::new("/k/Big.epub");
        let big = |bytes| Oversized {
            path: file.to_path_buf(),
            bytes,
        };
        let files = [big(96_400_000), big(212_000_000)];
        assert_eq!(Delivery::Email.over(&files).len(), 2);
        assert_eq!(Delivery::Other.over(&files), [big(212_000_000)]);
        let fix = levers_line(
            &Levers {
                split_bytes: 45_000_000,
                chapters_per_book: None,
                jpeg_quality: None,
            },
            Fewer::Every,
        );
        assert_eq!(
            too_big(file, 96_400_000, Delivery::Email, &fix),
            "warning: /k/Big.epub is 96.4 MB, over the 50 MB Send-to-Kindle email limit; \
             to fit: --split-mb 45 (parts), no JPEG quality down to 50 fits"
        );
        let planned = Levers {
            split_bytes: 180_000_000,
            chapters_per_book: Some(4),
            jpeg_quality: Some(80),
        };
        let spec = BudgetSpec {
            device: mangas_core::export::profile("kindle-paperwhite12").unwrap(),
            cut: Default::default(),
            page: Default::default(),
            delivery: Delivery::Other,
            format: mangas_core::export::Format::Epub,
            split: None,
        };
        let fix = levers_text(Some(&planned), &[], spec, Fewer::Every).unwrap();
        assert_eq!(
            too_big(file, 237_000_000, Delivery::Other, &fix),
            "warning: /k/Big.epub is 237.0 MB, over the 200 MB Send-to-Kindle web limit; \
             to fit: --every 4 (fewer chapters per book), --split-mb 180 (parts), --quality 80 \
             (lower JPEG quality)"
        );
    }
}
