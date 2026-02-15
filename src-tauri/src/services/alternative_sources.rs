use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::services::lua_parser::{self, DepotInfo};
use crate::services::st_parser;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintedWasteDepot {
    pub depot_id: String,
    pub manifest_id: Option<String>,
    pub depot_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintedWasteResult {
    pub depots: Vec<PrintedWasteDepot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelOsResult {
    pub files: Vec<String>,
    pub target_dir: String,
    pub depots: Vec<DepotInfo>,
}

/// Download from PrintedWaste API.
///
/// API: `GET https://gcore.api.printedwaste.com/app/{app_id}/depot`
/// Auth header: `Authorization: Bearer dGhpc19pcyBhX3JhbmRvbV90b2tlbg==`
pub async fn download_from_printed_waste(
    client: &Client,
    app_id: &str,
) -> Result<PrintedWasteResult, String> {
    let url = format!(
        "https://gcore.api.printedwaste.com/app/{}/depot",
        app_id
    );

    let response = client
        .get(&url)
        .header("Authorization", "Bearer dGhpc19pcyBhX3JhbmRvbV90b2tlbg==")
        .header("User-Agent", "SteamManifestDownloader")
        .send()
        .await
        .map_err(|e| format!("PrintedWaste API request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "PrintedWaste API error: {} {}",
            response.status(),
            response.status().canonical_reason().unwrap_or("")
        ));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse PrintedWaste response: {}", e))?;

    // Parse the depot info from response
    let mut depots = Vec::new();

    if let Some(depot_array) = data.as_array() {
        for depot_obj in depot_array {
            let depot_id = depot_obj
                .get("depot_id")
                .or_else(|| depot_obj.get("depotId"))
                .and_then(|v| v.as_str().or_else(|| v.as_u64().map(|_| "")).map(|s| {
                    if s.is_empty() {
                        v.as_u64().map(|n| n.to_string()).unwrap_or_default()
                    } else {
                        s.to_string()
                    }
                }))
                .unwrap_or_default();

            let manifest_id = depot_obj
                .get("manifest_id")
                .or_else(|| depot_obj.get("manifestId"))
                .and_then(|v| v.as_str().map(String::from).or_else(|| v.as_u64().map(|n| n.to_string())));

            let depot_key = depot_obj
                .get("depot_key")
                .or_else(|| depot_obj.get("depotKey"))
                .or_else(|| depot_obj.get("decryption_key"))
                .and_then(|v| v.as_str().map(String::from));

            if !depot_id.is_empty() {
                depots.push(PrintedWasteDepot {
                    depot_id,
                    manifest_id,
                    depot_key,
                });
            }
        }
    }

    Ok(PrintedWasteResult { depots })
}

/// Download from KernelOS and extract .lua and .st files.
///
/// Step 1: `GET https://kernelosgithub.onrender.com/get_signed_url/{app_id}` → get signed URL
/// Step 2: Download zip from signed URL
/// Step 3: Extract zip to temp dir using `zip` crate
/// Step 4: Find `.lua` and `.st` files in extracted content
/// Step 5: Parse found files with lua_parser / st_parser
pub async fn download_from_kernel_os(
    client: &Client,
    app_id: &str,
    output_dir: &Path,
) -> Result<KernelOsResult, String> {
    // Step 1: Get the signed download URL
    let api_url = format!(
        "https://kernelosgithub.onrender.com/get_signed_url/{}",
        app_id
    );

    let api_response = client
        .get(&api_url)
        .header("User-Agent", "SteamManifestDownloader")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("KernelOS API request failed: {}", e))?;

    let data: serde_json::Value = api_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse KernelOS API response: {}", e))?;

    let raw_url = data["url"]
        .as_str()
        .ok_or_else(|| format!("KernelOS: no download URL returned for app {}", app_id))?;

    let download_url = if raw_url.starts_with('/') {
        format!("https://kernelosgithub.onrender.com{}", raw_url)
    } else {
        raw_url.to_string()
    };

    // Step 2: Download the zip file
    let zip_response = client
        .get(&download_url)
        .header("User-Agent", "SteamManifestDownloader")
        .send()
        .await
        .map_err(|e| format!("KernelOS zip download failed: {}", e))?;

    if !zip_response.status().is_success() {
        return Err(format!(
            "KernelOS zip download error: {}",
            zip_response.status()
        ));
    }

    let zip_bytes = zip_response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read KernelOS zip response: {}", e))?;

    // Step 3: Create temp dir and extract zip
    let temp_dir = output_dir.join(format!("kernelos_{}", app_id));
    tokio::fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    // Extract zip using zip crate (blocking, use spawn_blocking)
    let zip_bytes_clone = zip_bytes.to_vec();
    let temp_dir_clone = temp_dir.clone();

    let extracted_files = tokio::task::spawn_blocking(move || {
        extract_zip_files(&zip_bytes_clone, &temp_dir_clone)
    })
    .await
    .map_err(|e| format!("Zip extraction task failed: {}", e))?
    .map_err(|e| format!("Zip extraction failed: {}", e))?;

    // Step 4 & 5: Find and parse .lua and .st files
    let mut all_depots: Vec<DepotInfo> = Vec::new();
    let mut file_paths: Vec<String> = Vec::new();

    for file_path in &extracted_files {
        file_paths.push(file_path.to_string_lossy().to_string());

        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "lua" => {
                // Read and parse lua file
                match tokio::fs::read_to_string(file_path).await {
                    Ok(content) => {
                        let result = lua_parser::parse_lua_file(&content);
                        all_depots.extend(result.depots);
                    }
                    Err(e) => {
                        eprintln!("[KernelOS] Failed to read lua file {:?}: {}", file_path, e);
                    }
                }
            }
            "st" => {
                // Read and parse st file
                match tokio::fs::read(file_path).await {
                    Ok(buffer) => {
                        match st_parser::parse_st_file(&buffer) {
                            Ok(result) => {
                                all_depots.extend(result.depots);
                            }
                            Err(e) => {
                                eprintln!(
                                    "[KernelOS] Failed to parse st file {:?}: {}",
                                    file_path, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[KernelOS] Failed to read st file {:?}: {}", file_path, e);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(KernelOsResult {
        files: file_paths,
        target_dir: temp_dir.to_string_lossy().to_string(),
        depots: all_depots,
    })
}

/// Extract .lua, .st, and .manifest files from a zip buffer to a target directory.
fn extract_zip_files(zip_bytes: &[u8], target_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to open zip archive: {}", e))?;

    let mut extracted_files = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry {}: {}", i, e))?;

        if file.is_dir() {
            continue;
        }

        let name = file.name().to_string();
        let ext = Path::new(&name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if !["manifest", "lua", "st"].contains(&ext.as_str()) {
            continue;
        }

        // Use just the filename (no subdirectories) to avoid path issues
        let filename = Path::new(&name)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or(name.clone());

        let output_path = target_dir.join(&filename);

        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| format!("Failed to read zip entry data: {}", e))?;

        std::fs::write(&output_path, &data)
            .map_err(|e| format!("Failed to write extracted file: {}", e))?;

        extracted_files.push(output_path);
    }

    Ok(extracted_files)
}
