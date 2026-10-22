//! Tauri shell. Commands in `commands.rs` are thin: parse, call `mangas_core`, return JSON.

mod commands;
mod first_run;
mod paths;
mod window_fit;
mod window_normal;
mod window_saved;
mod window_state;
mod window_theme;

pub fn run() {
    let fresh = first_run::is_fresh_install();
    init_logging();
    match paths::rate_limits() {
        Ok(dir) => mangas_providers::HostLimiters::shared().share_across_processes(dir),
        Err(e) => tracing::warn!("rate limits stay per process: {e}"),
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(window_state::plugin())
        .plugin(window_normal::plugin())
        .manage(commands::app_update::PendingUpdate::default())
        .setup(move |app| {
            window_state::restore(app.handle());
            allow_covers(app.handle());
            allow_cut_previews(app.handle());
            start_downloads(app.handle());
            if fresh {
                first_run::mark_setup_pending();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window_theme::show_themed,
            commands::app_update::check_app_update,
            commands::app_update::install_app_update,
            commands::list_library,
            commands::list_devices,
            commands::default_data_dir,
            commands::get_settings,
            commands::set_settings,
            commands::install_cli,
            commands::providers::list_providers,
            commands::providers::provider_status,
            commands::providers::test_proxy,
            commands::providers::proxy_in_use,
            commands::providers::connect_provider,
            commands::providers::disconnect_provider,
            commands::catalog::search_all,
            commands::catalog::follow_manga,
            commands::catalog::get_manga,
            commands::catalog::unfollow_manga,
            commands::catalog::list_chapters,
            commands::downloads::download_chapters,
            commands::downloads::list_downloads,
            commands::downloads::pause_download,
            commands::downloads::resume_download,
            commands::downloads::cancel_download,
            commands::downloads::retry_download,
            commands::downloads::clear_downloads,
            commands::downloads::dismiss_download,
            commands::downloads::queue_status,
            commands::folders::open_folder,
            commands::covers::cover_path,
            commands::organisation::list_entries,
            commands::organisation::list_categories,
            commands::organisation::create_category,
            commands::organisation::rename_category,
            commands::organisation::delete_category,
            commands::organisation::set_manga_categories,
            commands::organisation::set_reading_status,
            commands::import::import_local,
            commands::estimate::estimate_export,
            commands::estimate::fit_levers,
            commands::export::export_chapters,
            commands::detectors::detectors_status,
            commands::detectors::download_detectors,
            commands::detectors::cancel_detectors_download,
            commands::detectors::remove_detectors,
            commands::cut_preview::get_export_preset,
            commands::cut_preview::set_export_preset,
            commands::cut_preview::clear_export_preset,
            commands::cut_preview::preview_cut,
            commands::cut_preview::cancel_cut_preview,
            commands::volumes::plan_volumes,
            commands::volumes::export_volumes,
            commands::volumes::free_export_names,
            commands::volumes::check_export_dir,
            commands::volumes::export_file_name,
            commands::send::find_kindles,
            commands::send::usb_kindles,
            commands::send_formats::usb_verdict,
            commands::send_formats::email_refusal,
            commands::send::send_usb,
            commands::send::send_email,
            commands::send::send_test_email,
            commands::smtp_password::smtp_password_status,
            commands::smtp_password::set_smtp_password,
            commands::send::list_exports,
            commands::diagnostics::diagnostics,
            commands::diagnostics::log_ui_error,
            commands::storage::storage_usage,
            commands::storage::delete_downloads,
            commands::storage::clear_cache,
            commands::storage::move_data_dir,
            commands::storage::move_target_partial,
            commands::storage::remove_partial_move,
            commands::storage::export_backup,
            commands::storage::import_backup,
            commands::updates::check_updates,
            commands::updates::mark_viewed,
            commands::updates::set_auto_download,
            commands::updates::set_chapter_group
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Starts the download queue worker: jobs left from a previous run resume, and every change
/// reaches the UI as a [`commands::downloads::QUEUE_EVENT`].
fn start_downloads(app: &tauri::AppHandle) {
    use std::sync::Arc;
    use tauri::{Emitter, Manager};
    let runner = mangas_providers::QueueRunner::new(Box::new(|settings| {
        paths::downloads(settings).map_err(mangas_core::Error::Other)
    }));
    let queue = paths::library_db()
        .and_then(|db| mangas_core::library::open(&db).map_err(|e| e.to_string()))
        .map(|lib| {
            let now = mangas_core::download::queue::now_ms();
            match mangas_core::download::queue::reclaim_orphans(&lib, now) {
                Ok(back) if !back.is_empty() => {
                    tracing::info!("{} orphaned download(s) queued again", back.len())
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("reclaim orphaned downloads: {e}"),
            }
            let config = mangas_core::download::queue::QueueConfig::default();
            Arc::new(
                mangas_core::download::queue::Queue::new(Arc::new(lib), Arc::new(runner), config)
                    .following_settings(),
            )
        });
    if let Ok(queue) = &queue {
        let (queue, handle) = (queue.clone(), app.clone());
        tauri::async_runtime::spawn(async move {
            let emit = |event: mangas_core::download::queue::QueueEvent| {
                if let Err(e) = handle.emit(commands::downloads::QUEUE_EVENT, &event) {
                    tracing::warn!("emit download event: {e}");
                }
            };
            let scope = mangas_core::download::queue::Scope::All;
            if let Err(e) = queue.run(scope, false, &emit).await {
                tracing::error!("download queue stopped: {e}");
            }
        });
    } else if let Err(e) = &queue {
        tracing::error!("download queue unavailable: {e}");
    }
    app.manage(commands::downloads::Downloads(queue));
}

/// Lets the webview load cached covers through the asset protocol, and nothing else on disk.
fn allow_covers(app: &tauri::AppHandle) {
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
fn allow_cut_previews(app: &tauri::AppHandle) {
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
