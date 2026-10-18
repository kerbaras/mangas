//! Which smart-cut preview of each options panel is current, and which panels closed.

use std::collections::{HashMap, VecDeque};

/// Closed panels remembered. A request arriving after this many more closes isn't refused; it
/// only runs its own preview to the end, as no open panel shares its state.
const CLOSED_KEPT: usize = 32;

#[derive(Default)]
pub(super) struct Panels {
    latest: HashMap<String, u64>,
    closed: VecDeque<String>,
}

impl Panels {
    /// Makes `generation` the panel's current preview unless a newer one already is; `false` when
    /// the panel already closed.
    pub(super) fn start(&mut self, panel: &str, generation: u64) -> bool {
        if self.closed.iter().any(|p| p == panel) {
            return false;
        }
        let latest = self.latest.entry(panel.to_owned()).or_insert(generation);
        *latest = (*latest).max(generation);
        true
    }

    pub(super) fn is_open(&self, panel: &str) -> bool {
        self.latest.contains_key(panel)
    }

    /// Whether `generation` is still the panel's newest preview and the panel is open.
    pub(super) fn is_current(&self, panel: &str, generation: u64) -> bool {
        self.latest.get(panel) == Some(&generation)
    }

    pub(super) fn close(&mut self, panel: String) {
        self.latest.remove(&panel);
        self.closed.push_back(panel);
        if self.closed.len() > CLOSED_KEPT {
            self.closed.pop_front();
        }
    }
}

/// A panel id is a folder name under the preview cache: a UUID from the webview, nothing else.
pub(super) fn valid_panel(panel: &str) -> bool {
    !panel.is_empty()
        && panel.len() <= 64
        && panel.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newer_preview_of_the_same_panel_replaces_the_older() {
        let mut p = Panels::default();
        assert!(p.start("a", 1));
        assert!(p.start("a", 2));
        assert!(!p.is_current("a", 1));
        assert!(p.is_current("a", 2));
    }

    #[test]
    fn an_older_preview_arriving_late_does_not_replace_the_newer() {
        let mut p = Panels::default();
        assert!(p.start("a", 2));
        assert!(p.start("a", 1));
        assert!(p.is_current("a", 2));
        assert!(!p.is_current("a", 1));
    }

    #[test]
    fn panels_do_not_cancel_each_other() {
        let mut p = Panels::default();
        assert!(p.start("a", 1));
        assert!(p.start("b", 2));
        p.close("b".into());
        assert!(p.is_current("a", 1));
    }

    #[test]
    fn closing_stops_the_running_preview_and_refuses_late_ones() {
        let mut p = Panels::default();
        assert!(p.start("a", 1));
        p.close("a".into());
        assert!(!p.is_current("a", 1));
        assert!(!p.is_open("a"));
        assert!(!p.start("a", 2));
        assert!(!p.is_current("a", 2));
    }

    #[test]
    fn only_uuid_like_ids_name_a_folder() {
        assert!(valid_panel("0b6c7a8e-1f2d-4c3b-9a8e-7f6d5c4b3a21"));
        for bad in ["", "..", "../x", "a/b", "a b"] {
            assert!(!valid_panel(bad), "{bad:?}");
        }
    }
}
