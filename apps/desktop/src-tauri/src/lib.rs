//! Tauri shell. Commands in `commands.rs` are thin: parse, call `mangas_core`, return JSON.

mod commands;
mod paths;

pub fn run() {
    env_logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_library,
            commands::list_devices,
            commands::data_dir,
            commands::get_settings,
            commands::set_settings,
            commands::catalog::search_manga,
            commands::catalog::follow_manga,
            commands::catalog::unfollow_manga,
            commands::catalog::list_chapters,
            commands::catalog::download_chapters,
            commands::kindle::export_kindle,
            commands::kindle::detectors_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
