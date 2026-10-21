//! Which pages of a chapter can go through the pipeline side by side, read from file headers so
//! no page is decoded before its turn.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::IoAt;
use crate::domain::Result;
use crate::smartcut::is_long_strip;

/// Side margins to crop, by image width (see `strip_margins`).
pub(super) type Margins = HashMap<u32, (u32, u32)>;

/// Width and height of a page once its side margins are cropped, as `crop_sides` gives them.
pub(super) fn page_size(file: &Path, margins: &Margins) -> Result<(u32, u32)> {
    let (w, h) = image::ImageReader::open(file)
        .at(file)?
        .with_guessed_format()
        .at(file)?
        .into_dimensions()?;
    Ok(match margins.get(&w) {
        Some(&(left, right)) if left > 0 || right > 0 => (w.saturating_sub(left + right).max(1), h),
        _ => (w, h),
    })
}

/// How many of `files` from the start are regular pages (not long strips), at most `max`. Reads
/// headers only up to the first long strip, so a broken file past it fails when its turn comes.
pub(super) fn regular_run(
    files: &[PathBuf],
    margins: &Margins,
    device: (u32, u32),
    max: usize,
) -> Result<usize> {
    count_regular(files.iter().map(|f| page_size(f, margins)), device, max)
}

fn count_regular(
    sizes: impl Iterator<Item = Result<(u32, u32)>>,
    device: (u32, u32),
    max: usize,
) -> Result<usize> {
    let mut n = 0;
    for size in sizes.take(max) {
        let (w, h) = size?;
        if is_long_strip(w, h, device.0, device.1) {
            break;
        }
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_stops_at_the_first_long_strip_or_the_cap() {
        let device = (1000, 1500);
        let sizes = || [(800, 1200), (800, 1100), (800, 9000), (800, 1200)].map(Ok);
        let count = |skip, max| count_regular(sizes().into_iter().skip(skip), device, max).ok();
        assert_eq!(count(0, 8), Some(2));
        assert_eq!(count(0, 1), Some(1));
        assert_eq!(count(2, 8), Some(0));
    }

    #[test]
    fn headers_past_the_run_are_not_read() {
        let device = (1000, 1500);
        let broken = Err(crate::domain::Error::Other("unreadable".into()));
        let sizes = [Ok((800, 1200)), Ok((800, 9000)), broken];
        assert_eq!(count_regular(sizes.into_iter(), device, 8).ok(), Some(1));
        let sizes = [
            Ok((800, 1200)),
            Err(crate::domain::Error::Other("x".into())),
        ];
        assert!(count_regular(sizes.into_iter(), device, 8).is_err());
    }
}
