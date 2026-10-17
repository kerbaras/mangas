//! Tauri shell. Commands in `commands.rs` are thin: parse, call `mangas_core`, return JSON.

mod commands;
mod paths;

pub fn run() {
    init_logging();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_library,
            commands::list_devices,
            commands::default_data_dir,
            commands::get_settings,
            commands::set_settings,
            commands::install_cli,
            commands::providers::list_providers,
            commands::providers::provider_status,
            commands::providers::connect_provider,
            commands::providers::disconnect_provider,
            commands::catalog::search_manga,
            commands::catalog::follow_manga,
            commands::catalog::unfollow_manga,
            commands::catalog::list_chapters,
            commands::catalog::download_chapters,
            commands::import::import_local,
            commands::kindle::export_kindle,
            commands::kindle::detectors_status,
            commands::diagnostics::diagnostics,
            commands::diagnostics::open_logs,
            commands::diagnostics::log_ui_error
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// File logging under the platform data dir; the app still starts if the folder is unwritable.
fn init_logging() {
    let started = paths::log_dir()
        .and_then(|dir| mangas_core::logging::init_file(&dir).map_err(|e| e.to_string()));
    match started {
        Ok(()) => tracing::info!(
            version = env!("CARGO_PKG_VERSION"),
            os = std::env::consts::OS,
            arch = std::env::consts::ARCH,
            "mangas started"
        ),
        Err(e) => eprintln!("logging disabled: {e}"),
    }
}
