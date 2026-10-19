//! The window's own background, painted before the webview loads `index.html`: on the saved
//! theme's canvas colour, a dark-theme cold start shows no white frame before `theme-boot.js` runs.

use mangas_core::library::Theme;
use tauri::{window::Color, Runtime, WebviewWindow};

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
    if let Err(e) = window.set_background_color(Some(canvas(is_dark(theme, os)))) {
        tracing::warn!("set the window background: {e}");
    }
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
