//! Where a restored window may sit: fully inside one monitor's work area (#137).

use serde::{Deserialize, Serialize};

/// A screen rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    fn right(&self) -> i64 {
        i64::from(self.x) + i64::from(self.width)
    }

    fn bottom(&self) -> i64 {
        i64::from(self.y) + i64::from(self.height)
    }

    fn contains(&self, other: &Rect) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    fn overlap(&self, other: &Rect) -> i64 {
        let w = self.right().min(other.right()) - i64::from(self.x.max(other.x));
        let h = self.bottom().min(other.bottom()) - i64::from(self.y.max(other.y));
        w.max(0) * h.max(0)
    }
}

/// A side too long for its work area becomes 90% of it: a window exactly the size of the work
/// area is taken for a maximised one by window managers (KWin maximises it).
fn shrink(len: u32, span: u32) -> u32 {
    if len < span {
        len
    } else {
        (u64::from(span) * 9 / 10) as u32
    }
}

/// Where a `width`×`height` window with no saved state opens: centred in `area`, shrunk to fit.
pub fn opening(width: u32, height: u32, area: &Rect) -> Rect {
    let (width, height) = (shrink(width, area.width), shrink(height, area.height));
    let offset = |start: i32, span: u32, len: u32| {
        (i64::from(start) + (i64::from(span) - i64::from(len)) / 2) as i32
    };
    Rect {
        x: offset(area.x, area.width, width),
        y: offset(area.y, area.height, height),
        width,
        height,
    }
}

/// The rect `window` should move to, or `None` when it already sits inside a work area (or
/// there are none). Otherwise it goes into the area it overlaps most (the first one when it
/// overlaps none), shrunk to fit, keeping its position where it can.
pub fn fit(window: Rect, areas: &[Rect]) -> Option<Rect> {
    if areas.iter().any(|a| a.contains(&window)) {
        return None;
    }
    let area = areas
        .iter()
        .filter(|a| a.overlap(&window) > 0)
        .max_by_key(|a| a.overlap(&window))
        .or_else(|| areas.first())?;
    let width = shrink(window.width, area.width);
    let height = shrink(window.height, area.height);
    let clamp = |at: i32, start: i32, span: u32, len: u32| {
        let last = i64::from(start) + i64::from(span) - i64::from(len);
        i64::from(at).clamp(i64::from(start), last) as i32
    };
    Some(Rect {
        x: clamp(window.x, area.x, area.width, width),
        y: clamp(window.y, area.y, area.height, height),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn rect(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    const SCREEN: Rect = rect(0, 0, 1280, 800);

    #[test]
    fn a_window_inside_a_work_area_stays() {
        assert_eq!(fit(rect(100, 50, 1000, 700), &[SCREEN]), None);
        assert_eq!(fit(SCREEN, &[SCREEN]), None);
    }

    #[test]
    fn a_window_far_off_screen_comes_back() {
        assert_eq!(
            fit(rect(5000, 5000, 1000, 700), &[SCREEN]),
            Some(rect(280, 100, 1000, 700))
        );
        assert_eq!(
            fit(rect(-4000, -3000, 1000, 700), &[SCREEN]),
            Some(rect(0, 0, 1000, 700))
        );
    }

    #[test]
    fn a_window_mostly_off_screen_is_pulled_in_and_shrunk() {
        // #137: 1280×840 at (960, 758) on a 1280×840 screen.
        let screen = rect(0, 0, 1280, 840);
        assert_eq!(
            fit(rect(960, 758, 1280, 840), &[screen]),
            Some(rect(128, 84, 1152, 756))
        );
        assert_eq!(
            fit(rect(200, 100, 1400, 900), &[SCREEN]),
            Some(rect(128, 80, 1152, 720))
        );
        assert_eq!(
            fit(rect(960, 600, 1600, 1000), &[rect(0, 0, 1280, 756)]),
            Some(rect(128, 76, 1152, 680))
        );
    }

    #[test]
    fn the_monitor_it_overlaps_most_wins() {
        let left = rect(0, 0, 1920, 1080);
        let right = rect(1920, 0, 2560, 1440);
        let window = rect(1800, 100, 1200, 800);
        assert_eq!(
            fit(window, &[left, right]),
            Some(rect(1920, 100, 1200, 800))
        );
        assert_eq!(fit(rect(2000, 100, 1200, 800), &[left, right]), None);
    }

    #[test]
    fn a_new_window_opens_centred_and_fits_a_small_screen() {
        let area = rect(0, 0, 1280, 756);
        assert_eq!(opening(1000, 600, &area), rect(140, 78, 1000, 600));
        let config = opening(1280, 840, &area);
        assert_eq!(config, rect(64, 38, 1152, 680));
        assert_eq!(fit(config, &[area]), None);
        let right = rect(1920, 0, 2560, 1440);
        assert_eq!(opening(1280, 840, &right), rect(2560, 300, 1280, 840));
    }

    #[test]
    fn no_monitors_means_no_move() {
        assert_eq!(fit(rect(5000, 5000, 100, 100), &[]), None);
    }
}
