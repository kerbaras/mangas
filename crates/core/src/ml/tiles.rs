//! Tiling of long strips into square windows the detectors can see at once.

/// Vertical windows `[y0, y1)` of at most `tile` rows, each overlapping the previous by `overlap`.
pub fn windows(height: usize, tile: usize, overlap: usize) -> Vec<(usize, usize)> {
    if height == 0 || tile == 0 {
        return Vec::new();
    }
    if height <= tile {
        return vec![(0, height)];
    }
    let step = tile.saturating_sub(overlap).max(1);
    let mut out = Vec::new();
    let mut y = 0;
    loop {
        let y1 = (y + tile).min(height);
        out.push((y1.saturating_sub(tile), y1));
        if y1 == height {
            return out;
        }
        y += step;
    }
}
