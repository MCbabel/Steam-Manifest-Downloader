use reqwest::Client;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Download a manifest file from the ManifestHub API.
///
/// API URL: `https://api.manifesthub1.filegear-sg.me/manifest?apikey={key}&depotid={depot_id}&manifestid={manifest_id}`
///
/// Important: Buffer the response body once to avoid consuming it twice (learned from the Electron bug).
pub async fn download_from_manifest_hub(
    client: &Client,
    app_id: &str,
    depot_id: &str,
    manifest_id: &str,
    output_dir: &Path,
    api_key: &str,
) -> Result<PathBuf, String> {
    let url = format!(
        "https://api.manifesthub1.filegear-sg.me/manifest?apikey={}&depotid={}&manifestid={}",
        api_key, depot_id, manifest_id
    );

    let filename = format!("{}_{}.manifest", depot_id, manifest_id);

    // Ensure output directory exists
    fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let output_path = output_dir.join(&filename);

    let response = client
        .get(&url)
        .header("User-Agent", "SteamManifestDownloader")
        .send()
        .await
        .map_err(|e| format!("ManifestHub API request failed for depot {}: {}", depot_id, e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!(
            "ManifestHub API error for depot {}: {} {}{}",
            depot_id,
            status,
            status.canonical_reason().unwrap_or(""),
            if error_text.is_empty() {
                String::new()
            } else {
                format!(" - {}", error_text)
            }
        ));
    }

    // Buffer the entire response body once
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read ManifestHub response body: {}", e))?;

    // Check if the response is a JSON error
    if content_type.contains("application/json") {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            let error_msg = json
                .get("error")
                .or_else(|| json.get("message"))
                .and_then(|v| v.as_str());
            if let Some(msg) = error_msg {
                return Err(format!("ManifestHub API: {}", msg));
            }
        }
    }

    // Write binary response to file
    fs::write(&output_path, &bytes)
        .await
        .map_err(|e| format!("Failed to write manifest file: {}", e))?;

    // app_id is available for context but not needed in the URL
    let _ = app_id;

    Ok(output_path)
}
