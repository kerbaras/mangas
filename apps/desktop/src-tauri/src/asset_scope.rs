//! What the webview may load from disk through the asset protocol.

use crate::paths;

/// Lets the webview load cached covers through the asset protocol, and nothing else on disk.
pub fn allow_covers(app: &tauri::AppHandle) {
    use tauri::Manager;
    let allowed = paths::covers().and_then(|dir| {
        app.asset_protocol_scope()
            .allow_directory(&dir, false)
            .map_err(|e| e.to_string())
    });
    if let Err(e) = allowed {
        tracing::warn!("covers will show placeholders: {e}");
    }
}

/// Starts the preview cache empty and lets the webview load its pages.
pub fn allow_cut_previews(app: &tauri::AppHandle) {
    use tauri::Manager;
    let allowed = paths::cut_previews().and_then(|dir| {
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            tracing::debug!("cut preview cache: {e}");
        }
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        app.asset_protocol_scope()
            .allow_directory(&dir, true)
            .map_err(|e| e.to_string())
    });
    if let Err(e) = allowed {
        tracing::warn!("smart-cut previews will not show: {e}");
    }
}
