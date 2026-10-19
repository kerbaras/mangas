//! Which pages of a chapter can go through the pipeline side by side, read from file headers so
//! no page is decoded before its turn.

use std::collections::HashMap;
use std::path::Path;

use crate::domain::Result;
use crate::smartcut::is_long_strip;

/// Side margins to crop, by image width (see `strip_margins`).
pub(super) type Margins = HashMap<u32, (u32, u32)>;

/// Width and height of a page once its side margins are cropped, as `crop_sides` gives them.
pub(super) fn page_size(file: &Path, margins: &Margins) -> Result<(u32, u32)> {
    let (w, h) = image::ImageReader::open(file)?
        .with_guessed_format()?
        .into_dimensions()?;
    Ok(match margins.get(&w) {
        Some(&(left, right)) if left > 0 || right > 0 => (w.saturating_sub(left + right).max(1), h),
        _ => (w, h),
    })
}

/// How many pages at the start of `sizes` are regular pages (not long strips), at most `max`.
pub(super) fn regular_run(sizes: &[(u32, u32)], device: (u32, u32), max: usize) -> usize {
    sizes
        .iter()
        .take(max)
        .take_while(|&&(w, h)| !is_long_strip(w, h, device.0, device.1))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_stops_at_the_first_long_strip_or_the_cap() {
        let device = (1000, 1500);
        let sizes = [(800, 1200), (800, 1100), (800, 9000), (800, 1200)];
        assert_eq!(regular_run(&sizes, device, 8), 2);
        assert_eq!(regular_run(&sizes, device, 1), 1);
        assert_eq!(regular_run(&sizes[2..], device, 8), 0);
    }
}
