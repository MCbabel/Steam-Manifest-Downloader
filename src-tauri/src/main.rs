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
            // Initialize app data directory
            let app_data = app.path().app_data_dir().expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data).ok();
            
            // Initialize services state
            let state = services::AppState::new(app.handle().clone());
            app.manage(state);

            // On Windows, remove native decorations so the custom title bar is used.
            // On Linux, keep native decorations (set in tauri.conf.json) for proper
            // window drag / resize / close behavior.
            #[cfg(target_os = "windows")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_decorations(false);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // File operations
            commands::parse_lua_file,
            commands::parse_lua_content,
            // Search
            commands::search_repos,
            commands::get_repo_manifests,
            commands::search_alternative,
            // Steam
            commands::get_steam_app_info,
            // Download
            commands::start_download,
            commands::cancel_download,
            commands::export_batch_script,
            // Settings
            commands::get_settings,
            commands::save_settings,
            // System
            commands::check_dotnet,
            commands::get_disk_space,
            // Window
            commands::minimize_window,
            commands::maximize_window,
            commands::close_window,
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
