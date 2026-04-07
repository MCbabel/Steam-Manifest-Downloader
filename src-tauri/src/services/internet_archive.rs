use reqwest::Client;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const IA_BASE_URL: &str = "https://ia800607.us.archive.org/view_archive.php";
const IA_ARCHIVE_PATH: &str = "/33/items/manifest-hub-repo/branches.zip";

/// Build URL for a file inside the Internet Archive ZIP.
fn build_ia_url(app_id: &str, filename: &str) -> String {
    let file_path = format!("branches/{}/{}", app_id, filename);
    let encoded_path = file_path.replace("/", "%2F");
    format!(
        "{}?archive={}&file={}",
        IA_BASE_URL, IA_ARCHIVE_PATH, encoded_path
    )
}

/// Check if an App ID exists in the Internet Archive by trying to fetch its .lua file.
pub async fn check_app_exists(client: &Client, app_id: &str) -> Result<bool, String> {
    let url = build_ia_url(app_id, &format!("{}.lua", app_id));
    let response = client
        .head(&url)
        .header("User-Agent", "SteamManifestDownloader")
        .send()
        .await
        .map_err(|e| format!("Internet Archive request failed: {}", e))?;

    Ok(response.status().is_success())
}

/// Download a text file (like .lua or key.vdf) from the Internet Archive.
pub async fn download_text_file(
    client: &Client,
    app_id: &str,
    filename: &str,
) -> Result<String, String> {
    let url = build_ia_url(app_id, filename);
    let response = client
        .get(&url)
        .header("User-Agent", "SteamManifestDownloader")
        .send()
        .await
        .map_err(|e| format!("Internet Archive download failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "File not found: {} (HTTP {})",
            filename,
            response.status()
        ));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))
}

/// Download a binary file (like .manifest) from the Internet Archive to disk.
pub async fn download_manifest_file(
    client: &Client,
    app_id: &str,
    depot_id: &str,
    manifest_id: &str,
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let filename = format!("{}_{}.manifest", depot_id, manifest_id);
    let url = build_ia_url(app_id, &filename);

    let response = client
        .get(&url)
        .header("User-Agent", "SteamManifestDownloader")
        .send()
        .await
        .map_err(|e| format!("Manifest download failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Manifest not found: {} (HTTP {})",
            filename,
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    tokio::fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create dir: {}", e))?;

    let output_path = output_dir.join(&filename);
    tokio::fs::write(&output_path, &bytes)
        .await
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    Ok(output_path)
}

/// Extract depot sizes from the {AppID}.json file.
/// Maps (depot_id, manifest_gid) → size in bytes.
fn extract_depot_sizes(json: &serde_json::Value) -> HashMap<(String, String), u64> {
    let mut sizes = HashMap::new();

    let depot_obj = match json.get("depot").and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => return sizes,
    };

    for (depot_id, depot_value) in depot_obj {
        // Skip non-depot entries (like "depotdeltapatches", "baselanguages", etc.)
        let manifests = match depot_value.get("manifests").and_then(|v| v.as_object()) {
            Some(m) => m,
            None => continue,
        };

        for (_branch, branch_data) in manifests {
            let gid = match branch_data.get("gid").and_then(|v| v.as_str()) {
                Some(g) => g.to_string(),
                None => continue,
            };

            let size = branch_data
                .get("size")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);

            // Only store non-trivial sizes (skip size=7 which is empty/placeholder)
            if size > 100 {
                sizes.insert((depot_id.clone(), gid), size);
            }
        }
    }

    sizes
}

/// Download and parse the .lua file for an app, also try key.vdf.
/// Returns structured app data with manifests and depot keys.
pub async fn get_app_data(
    client: &Client,
    app_id: &str,
) -> Result<AppArchiveData, String> {
    // Download .lua file
    let lua_filename = format!("{}.lua", app_id);
    let lua_content = download_text_file(client, app_id, &lua_filename).await?;

    let lua_result = crate::services::lua_parser::parse_lua_file(&lua_content);

    // Try to download key.vdf (optional, may not exist)
    let mut depot_keys = HashMap::new();
    if let Ok(vdf_content) = download_text_file(client, app_id, "key.vdf").await {
        let vdf_keys =
            crate::services::vdf_parser::parse_key_vdf(&vdf_content, Some("InternetArchive"));
        depot_keys.extend(vdf_keys);
    }

    // Try to download {AppID}.json for depot sizes (optional, may not exist)
    let depot_sizes = match download_text_file(client, app_id, &format!("{}.json", app_id)).await {
        Ok(json_content) => {
            match serde_json::from_str::<serde_json::Value>(&json_content) {
                Ok(json) => extract_depot_sizes(&json),
                Err(e) => {
                    eprintln!("[InternetArchive] Failed to parse {}.json: {}", app_id, e);
                    HashMap::new()
                }
            }
        }
        Err(_) => HashMap::new(), // Graceful fallback: no sizes available
    };

    // Merge lua depot keys
    for depot in &lua_result.depots {
        if let Some(ref key) = depot.depot_key {
            depot_keys.insert(depot.depot_id.to_string(), key.clone());
        }
    }

    // Build manifest list from lua data
    let manifests: Vec<ManifestInfo> = lua_result
        .depots
        .iter()
        .filter_map(|d| {
            d.manifest_id.as_ref().map(|mid| {
                // Look up size for this depot+manifest combo
                let size_bytes = depot_sizes
                    .get(&(d.depot_id.to_string(), mid.clone()))
                    .copied();

                ManifestInfo {
                    depot_id: d.depot_id.to_string(),
                    manifest_id: mid.clone(),
                    depot_key: depot_keys.get(&d.depot_id.to_string()).cloned(),
                    size_bytes,
                }
            })
        })
        .collect();

    let has_key_vdf = !depot_keys.is_empty();

    Ok(AppArchiveData {
        app_id: app_id.to_string(),
        manifests,
        depot_keys,
        has_key_vdf,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManifestInfo {
    pub depot_id: String,
    pub manifest_id: String,
    pub depot_key: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppArchiveData {
    pub app_id: String,
    pub manifests: Vec<ManifestInfo>,
    pub depot_keys: HashMap<String, String>,
    pub has_key_vdf: bool,
}
