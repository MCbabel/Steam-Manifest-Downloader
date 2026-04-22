use std::path::PathBuf;
use tauri::{command, AppHandle, Manager};
use crate::services::history as history_service;

#[command]
pub async fn get_history(app: AppHandle) -> Result<serde_json::Value, String> {
    let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let history = history_service::load_history(&app_data_dir).await;
    serde_json::to_value(&history.entries).map_err(|e| format!("Failed to serialize history: {}", e))
}

#[command]
pub async fn remove_history_entry(app: AppHandle, entry_id: String) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    history_service::remove_entry(&app_data_dir, &entry_id).await
}

#[command]
pub async fn clear_history(app: AppHandle) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    history_service::clear(&app_data_dir).await
}

#[command]
pub async fn open_folder(path: String) -> Result<(), String> {
    let dir = std::path::Path::new(&path);
    if !dir.exists() {
        return Err("Directory does not exist".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(())
}
