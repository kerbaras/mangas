//! A window's normal (un-maximised) geometry, kept apart from the maximised flag (#194). The
//! window-state plugin keeps a maximised window's position in `prev_x/prev_y`, shifted on every
//! move, so the two moves KWin makes while maximising leave 0,0 there and un-maximising after a
//! restart lands in the corner. This records the rect only while the window is shown and neither
//! maximised, minimised nor fullscreen, and saves it on exit next to the plugin's file.

use std::{collections::HashMap, fs, path::PathBuf, sync::Mutex};

use tauri::{plugin::TauriPlugin, AppHandle, Manager, RunEvent, Runtime, Window, WindowEvent};

use crate::window_fit::Rect;

const FILE: &str = ".window-normal.json";

type Normals = HashMap<String, Rect>;

struct Tracked(Mutex<Normals>);

/// The rect a window in this state should be remembered with, if any.
fn normal_rect(normal: bool, x: i32, y: i32, width: u32, height: u32) -> Option<Rect> {
    (normal && width > 0 && height > 0).then_some(Rect {
        x,
        y,
        width,
        height,
    })
}

fn parse(json: &str) -> Normals {
    serde_json::from_str(json).unwrap_or_default()
}

fn file<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join(FILE))
}

fn observe<R: Runtime>(w: &Window<R>) -> Option<Rect> {
    let normal = w.is_visible().ok()?
        && !w.is_maximized().ok()?
        && !w.is_minimized().ok()?
        && !w.is_fullscreen().ok()?;
    let (at, size) = (w.outer_position().ok()?, w.inner_size().ok()?);
    normal_rect(normal, at.x, at.y, size.width, size.height)
}

fn record<R: Runtime>(w: &Window<R>) {
    let Some(rect) = observe(w) else {
        return;
    };
    if let Ok(mut all) = w.state::<Tracked>().0.lock() {
        all.insert(w.label().to_string(), rect);
    }
}

fn save<R: Runtime>(app: &AppHandle<R>) {
    let Some(path) = file(app) else {
        return;
    };
    let json = match app.state::<Tracked>().0.lock() {
        Ok(all) => serde_json::to_string_pretty(&*all),
        Err(_) => return,
    };
    let written = json
        .map_err(std::io::Error::other)
        .and_then(|json| fs::create_dir_all(path.parent().unwrap_or(&path)).map(|()| json))
        .and_then(|json| fs::write(&path, json));
    if let Err(e) = written {
        tracing::warn!("save the window's normal geometry: {e}");
    }
}

/// The normal rect saved for `label` (by this or an earlier run).
pub fn saved<R: Runtime>(app: &AppHandle<R>, label: &str) -> Option<Rect> {
    let tracked = app.try_state::<Tracked>()?;
    let all = tracked.0.lock().ok()?;
    all.get(label).copied()
}

/// Loads the saved rects, keeps each window's current one, and writes them back on exit; a
/// window that stays maximised all session keeps the rect it was saved with.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("window-normal")
        .setup(|app, _| {
            let all = file(app)
                .and_then(|f| fs::read_to_string(f).ok())
                .map(|json| parse(&json))
                .unwrap_or_default();
            app.manage(Tracked(Mutex::new(all)));
            Ok(())
        })
        .on_window_ready(|window| {
            let w = window.clone();
            window.on_window_event(move |e| {
                if matches!(e, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
                    record(&w);
                }
            });
        })
        .on_event(|app, e| {
            if let RunEvent::Exit = e {
                save(app);
            }
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_normal_window_with_a_size_is_remembered() {
        let rect = normal_rect(true, 400, 300, 2200, 1500);
        assert_eq!(
            rect.map(|r| (r.x, r.y, r.width, r.height)),
            Some((400, 300, 2200, 1500))
        );
        assert_eq!(normal_rect(false, 0, 0, 3200, 2356), None);
        assert_eq!(normal_rect(true, 0, 0, 0, 1500), None);
    }

    #[test]
    fn the_file_round_trips_and_a_broken_one_is_empty() {
        let mut all = Normals::new();
        all.insert(
            "main".into(),
            Rect {
                x: -20,
                y: 30,
                width: 1100,
                height: 700,
            },
        );
        let json = serde_json::to_string(&all).unwrap();
        assert_eq!(parse(&json), all);
        assert!(parse("not json").is_empty());
        assert!(parse(r#"{"main": {"x": 1}}"#).is_empty());
    }
}
