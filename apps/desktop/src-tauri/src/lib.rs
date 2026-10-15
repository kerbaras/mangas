//! Tauri shell. Commands in `commands.rs` are thin: parse, call `mangas_core`, return JSON.

mod commands;
mod paths;

pub fn run() {
    env_logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_library,
            commands::list_devices,
            commands::search_manga,
            commands::follow_manga,
            commands::unfollow_manga,
            commands::list_chapters,
            commands::download_chapters,
            commands::export_kindle,
            commands::detectors_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
