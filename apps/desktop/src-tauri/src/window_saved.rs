//! The geometry the window-state plugin saved for a window (`.window-state.json`).

use std::collections::HashMap;

use serde::Deserialize;

use crate::window_fit::Rect;

#[derive(Deserialize)]
struct Entry {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    #[serde(default)]
    prev_x: i32,
    #[serde(default)]
    prev_y: i32,
    #[serde(default)]
    maximized: bool,
}

/// The unmaximised rect a window had, and whether it was maximised.
#[derive(Debug, PartialEq, Eq)]
pub struct Saved {
    pub rect: Rect,
    pub maximized: bool,
}

/// What was saved for `label`; `None` without a usable entry (the plugin writes 0×0 defaults).
pub fn parse(json: &str, label: &str) -> Option<Saved> {
    let mut all: HashMap<String, Entry> = serde_json::from_str(json).ok()?;
    let e = all.remove(label)?;
    if e.width == 0 || e.height == 0 {
        return None;
    }
    // A maximised window keeps its unmaximised size; its position is in `prev_*`.
    let (x, y) = if e.maximized {
        (e.prev_x, e.prev_y)
    } else {
        (e.x, e.y)
    };
    Some(Saved {
        rect: Rect {
            x,
            y,
            width: e.width,
            height: e.height,
        },
        maximized: e.maximized,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"{
      "main": {"width": 1100, "height": 700, "x": 80, "y": 30, "prev_x": 5, "prev_y": 6,
               "maximized": false, "visible": true, "decorated": true, "fullscreen": false},
      "max": {"width": 1000, "height": 600, "x": 0, "y": 0, "prev_x": 40, "prev_y": 50,
              "maximized": true},
      "blank": {"width": 0, "height": 0, "x": 0, "y": 0}
    }"#;

    #[test]
    fn reads_the_saved_rect() {
        let rect = Rect {
            x: 80,
            y: 30,
            width: 1100,
            height: 700,
        };
        assert_eq!(
            parse(FILE, "main"),
            Some(Saved {
                rect,
                maximized: false
            })
        );
    }

    #[test]
    fn a_maximised_window_restores_from_its_previous_position() {
        let saved = parse(FILE, "max").unwrap();
        assert!(saved.maximized);
        assert_eq!((saved.rect.x, saved.rect.y), (40, 50));
        assert_eq!((saved.rect.width, saved.rect.height), (1000, 600));
    }

    #[test]
    fn missing_blank_or_broken_state_is_none() {
        assert_eq!(parse(FILE, "other"), None);
        assert_eq!(parse(FILE, "blank"), None);
        assert_eq!(parse("not json", "main"), None);
    }
}
