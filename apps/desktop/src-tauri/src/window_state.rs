//! Remembered window geometry. The window-state plugin saves size, position and the maximised
//! state, `window_normal` the un-maximised rect (#194); [`restore`] puts the main window back,
//! moved fully on-screen first (#137), then maximises it if it was.

use std::fs;

use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow};
use tauri_plugin_window_state::{StateFlags, DEFAULT_FILENAME};

use crate::window_fit::{fit, opening, Rect};
use crate::{window_normal, window_saved};

const MAIN: &str = "main";

/// The plugin only saves the main window: [`restore`] reads the saved numbers itself, because
/// checking the live window right after the plugin's restore races the window manager.
/// Decorations stay with the config: the custom title bar depends on them.
pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_window_state::Builder::new()
        .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED)
        .skip_initial_state(MAIN)
        .build()
}

fn saved<R: Runtime>(app: &AppHandle<R>) -> Option<window_saved::Saved> {
    let file = app.path().app_config_dir().ok()?.join(DEFAULT_FILENAME);
    window_saved::parse(&fs::read_to_string(file).ok()?, MAIN)
}

fn work_area(m: &Monitor) -> Rect {
    let area = m.work_area();
    Rect {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width,
        height: area.size.height,
    }
}

/// The config size centred on the primary screen (first launch, or no usable saved state). Read
/// from the config: a hidden window doesn't report its geometry reliably.
fn configured<R: Runtime>(
    app: &AppHandle<R>,
    window: &WebviewWindow<R>,
    areas: &[Rect],
) -> Option<Rect> {
    let config = app.config().app.windows.iter().find(|w| w.label == MAIN)?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let px = |logical: f64| (logical * scale).round() as u32;
    let primary = window
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| work_area(&m));
    let area = primary.or_else(|| areas.first().copied())?;
    Some(opening(px(config.width), px(config.height), &area))
}

/// The main window starts hidden: placed before it is first shown, a window manager never sees
/// the config size (which may not fit the screen, so KWin would maximise it).
pub fn restore<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN) else {
        return;
    };
    place(app, &window);
    if let Err(e) = window.show() {
        tracing::error!("show the main window: {e}");
    }
}

fn place<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) {
    let areas: Vec<Rect> = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(work_area)
        .collect();
    let plugin = saved(app);
    let maximized = plugin.as_ref().is_some_and(|s| s.maximized);
    let from = window_normal::saved(app, MAIN)
        .or(plugin.map(|s| s.rect))
        .or_else(|| configured(app, window, &areas));
    let Some(from) = from else {
        return;
    };
    let to = fit(from, &areas).unwrap_or(from);
    if to != from {
        tracing::info!(?from, ?to, "window was off-screen; moved back");
    }
    let placed = window
        .set_size(PhysicalSize::new(to.width, to.height))
        .and_then(|()| window.set_position(PhysicalPosition::new(to.x, to.y)))
        .and_then(|()| if maximized { window.maximize() } else { Ok(()) });
    if let Err(e) = placed {
        tracing::warn!("restore window geometry: {e}");
    }
}
