use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_download_location")]
    pub download_location: String,
    #[serde(default)]
    pub github_token: String,
    #[serde(default = "default_dd_extra_args")]
    pub dd_extra_args: Vec<String>,
}

fn default_download_location() -> String {
    // Default: ~/Documents/SteamDownloads
    if let Some(home) = dirs_next_home() {
        let docs = PathBuf::from(&home).join("Documents").join("SteamDownloads");
        return docs.to_string_lossy().to_string();
    }
    "./downloads".to_string()
}

fn dirs_next_home() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok()
    }
}

fn default_dd_extra_args() -> Vec<String> {
    vec![
        "-max-downloads".to_string(),
        "8".to_string(),
        "-verify-all".to_string(),
    ]
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_location: default_download_location(),
            github_token: String::new(),
            dd_extra_args: default_dd_extra_args(),
        }
    }
}

/// Get the settings file path within the app data directory.
fn settings_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("settings.json")
}

/// Load settings from `{app_data_dir}/settings.json`.
/// Returns default settings if the file doesn't exist or can't be parsed.
pub async fn load_settings(app_data_dir: &Path) -> Settings {
    let path = settings_path(app_data_dir);

    match fs::read_to_string(&path).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Save settings to `{app_data_dir}/settings.json`.
pub async fn save_settings(app_data_dir: &Path, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app_data_dir);

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create settings directory: {}", e))?;
    }

    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(&path, content)
        .await
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

    Ok(())
}
