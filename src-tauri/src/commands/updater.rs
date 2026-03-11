use std::path::PathBuf;
use tauri::{command, AppHandle, Manager};
use crate::services::settings as settings_service;

const GITHUB_REPO: &str = "MCbabel/Steam-Manifest-Downloader";
const USER_AGENT: &str = "SteamManifestDownloader";

/// Check GitHub for a newer release. Returns update info or null.
#[command]
pub async fn check_for_updates(app: AppHandle) -> Result<serde_json::Value, String> {
    let current_version = app.config().version.clone().unwrap_or_default();

    let client = reqwest::Client::new();
    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);

    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Failed to check for updates: {}", e))?;

    if !response.status().is_success() {
        return Ok(serde_json::json!({
            "available": false,
            "error": format!("GitHub API returned {}", response.status()),
        }));
    }

    let release: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let tag = release["tag_name"].as_str().unwrap_or("");
    let remote_version = tag.trim_start_matches('v');

    // Simple semver comparison
    if !is_newer_version(&current_version, remote_version) {
        return Ok(serde_json::json!({ "available": false }));
    }

    // Find the NSIS installer asset (.exe in the assets, not portable)
    let mut installer_url = None;
    if let Some(assets) = release["assets"].as_array() {
        for asset in assets {
            let name = asset["name"].as_str().unwrap_or("");
            if name.ends_with("-setup.exe") || name.contains("x64-setup") {
                installer_url = asset["browser_download_url"].as_str().map(String::from);
                break;
            }
        }
    }

    Ok(serde_json::json!({
        "available": true,
        "version": remote_version,
        "currentVersion": current_version,
        "date": release["published_at"].as_str(),
        "body": release["body"].as_str(),
        "installerUrl": installer_url,
        "releaseUrl": release["html_url"].as_str(),
    }))
}

/// Download the installer and run it, then exit the current app.
#[command]
pub async fn install_update(app: AppHandle, installer_url: String) -> Result<(), String> {
    let client = reqwest::Client::new();

    // Download to temp directory
    let temp_dir = std::env::temp_dir().join("SteamManifestDownloader");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let installer_path = temp_dir.join("update-installer.exe");

    let response = client
        .get(&installer_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Failed to download update: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status {}", response.status()));
    }

    let bytes = response.bytes().await
        .map_err(|e| format!("Failed to read download: {}", e))?;

    std::fs::write(&installer_path, &bytes)
        .map_err(|e| format!("Failed to save installer: {}", e))?;

    // Launch the installer and exit
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new(&installer_path)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, open the release page in the browser instead
        let _ = open::that(installer_url);
    }

    // Exit the app so the installer can replace files
    app.exit(0);
    Ok(())
}

/// Get the auto_update setting value.
#[command]
pub async fn get_auto_update_enabled(app: AppHandle) -> Result<bool, String> {
    let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let settings = settings_service::load_settings(&app_data_dir).await;
    Ok(settings.auto_update)
}

/// Simple semver comparison: returns true if remote > current
fn is_newer_version(current: &str, remote: &str) -> bool {
    let parse = |v: &str| -> (u64, u64, u64) {
        let parts: Vec<u64> = v.split('.').filter_map(|p| p.parse().ok()).collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };
    let c = parse(current);
    let r = parse(remote);
    r > c
}
