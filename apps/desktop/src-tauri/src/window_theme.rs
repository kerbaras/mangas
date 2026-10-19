//! The main window's first frame. It starts hidden, painted on the saved theme's canvas colour;
//! `public/theme-boot.js` shows it once the page has its palette (the webview's own reading of the
//! OS theme can differ from the native one), and a fallback shows it if the page never reports.

use std::{thread, time::Duration};

use mangas_core::library::Theme;
use tauri::{window::Color, Runtime, WebviewWindow};

/// How long the window waits for `theme-boot.js` before it is shown anyway.
const FALLBACK: Duration = Duration::from_secs(2);

/// `--color-canvas` in `src/styles/tokens.css`, light and dark.
const LIGHT: Color = Color(0xf7, 0xf7, 0xf5, 0xff);
const DARK: Color = Color(0x1f, 0x1f, 0x1e, 0xff);

/// Whether `theme` paints dark when the OS prefers `os`; `System` follows the OS like
/// `prefers-color-scheme` does in the webview.
pub fn is_dark(theme: Theme, os: tauri::Theme) -> bool {
    match theme {
        Theme::Dark => true,
        Theme::Light => false,
        _ => os == tauri::Theme::Dark,
    }
}

pub fn canvas(dark: bool) -> Color {
    if dark {
        DARK
    } else {
        LIGHT
    }
}

/// Paints `window` on the canvas colour of the theme stored in the settings.
pub fn paint<R: Runtime>(window: &WebviewWindow<R>) {
    let theme = crate::commands::settings_or_default()
        .theme
        .unwrap_or_default();
    let os = window.theme().unwrap_or(tauri::Theme::Light);
    set_canvas(window, is_dark(theme, os));
}

fn set_canvas<R: Runtime>(window: &WebviewWindow<R>, dark: bool) {
    if let Err(e) = window.set_background_color(Some(canvas(dark))) {
        tracing::warn!("set the window background: {e}");
    }
}

fn show<R: Runtime>(window: &WebviewWindow<R>) {
    if window.is_visible().unwrap_or(false) {
        return;
    }
    if let Err(e) = window.show() {
        tracing::error!("show the main window: {e}");
    }
}

/// Shows `window` after [`FALLBACK`] unless the page has shown it by then (a load error, a
/// missing script).
pub fn show_later<R: Runtime>(window: WebviewWindow<R>) {
    thread::spawn(move || {
        thread::sleep(FALLBACK);
        show(&window);
    });
}

/// From `theme-boot.js`, once the page has its palette: the window takes the page's canvas colour
/// and is shown.
#[tauri::command]
pub fn show_themed<R: Runtime>(window: WebviewWindow<R>, dark: bool) {
    set_canvas(&window, dark);
    show(&window);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_theme_wins_over_the_os() {
        assert!(is_dark(Theme::Dark, tauri::Theme::Light));
        assert!(!is_dark(Theme::Light, tauri::Theme::Dark));
    }

    #[test]
    fn system_follows_the_os() {
        assert!(is_dark(Theme::System, tauri::Theme::Dark));
        assert!(!is_dark(Theme::System, tauri::Theme::Light));
    }

    #[test]
    fn canvas_matches_the_tokens() {
        let tokens = include_str!("../../src/styles/tokens.css").to_lowercase();
        assert!(tokens.contains("--color-canvas: #f7f7f5"));
        assert!(tokens.contains("--color-canvas: #1f1f1e"));
        assert_eq!(canvas(true), DARK);
    }
}
