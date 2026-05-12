#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod services;

use tauri::{Emitter, Manager};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir().expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data).ok();

            let app_version = app.config().version.clone().unwrap_or_default();
            let channel = option_env!("SMD_BUILD_CHANNEL").unwrap_or("dev-local").to_string();

            let telemetry = services::telemetry::Telemetry::new(
                app_data.clone(),
                app_version,
                channel,
            );
            telemetry.clone().spawn_background_flush();

            let mut state = services::AppState::new(app.handle().clone());
            state.telemetry = Some(telemetry);
            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // File operations
            commands::parse_lua_file,
            commands::parse_lua_content,
            // Search
            commands::search_repos,
            commands::get_repo_manifests,
            commands::search_steam_games,
            // Steam
            commands::get_steam_app_info,
            // Download
            commands::start_download,
            commands::cancel_download,
            // Settings
            commands::get_settings,
            commands::save_settings,
            // System
            commands::check_dotnet,
            commands::get_disk_space,
            commands::get_build_info,
            // Window
            commands::minimize_window,
            commands::maximize_window,
            commands::close_window,
            commands::restart_app,
            // Updater
            commands::check_for_updates,
            commands::install_update,
            commands::get_auto_update_enabled,
            // History
            commands::get_history,
            commands::remove_history_entry,
            commands::clear_history,
            commands::open_folder,
            // Shortcuts
            commands::is_shortcut_supported,
            commands::detect_executables,
            commands::create_shortcuts,
            // Telemetry
            commands::get_telemetry_status,
            commands::set_telemetry_consent,
            commands::emit_telemetry_event,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<services::AppState>();
                if state.has_active_downloads() {
                    api.prevent_close();
                    let window = window.clone();
                    window.emit("close-requested", ()).ok();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
