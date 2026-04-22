use reqwest::Client;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const IA_BASE_URL: &str = "https://ia800607.us.archive.org/view_archive.php";

struct ArchiveSource {
    archive_path: &'static str,
    prefix: &'static str,
}

// Order = priority: branches.zip is checked first, fallback to NEW-depot-keys.
const ARCHIVES: &[ArchiveSource] = &[
    ArchiveSource {
        archive_path: "/33/items/manifest-hub-repo/branches.zip",
        prefix: "branches",
    },
    ArchiveSource {
        archive_path: "/33/items/manifest-hub-repo/NEW-depot-keys.zip",
        prefix: "NEW-depot-keys",
    },
];

fn build_url(source: &ArchiveSource, app_id: &str, filename: &str) -> String {
    let file_path = format!("{}/{}/{}", source.prefix, app_id, filename);
    let encoded_path = file_path.replace("/", "%2F");
    format!(
        "{}?archive={}&file={}",
        IA_BASE_URL, source.archive_path, encoded_path
    )
}

pub async fn check_app_exists(client: &Client, app_id: &str) -> Result<bool, String> {
    let lua_filename = format!("{}.lua", app_id);

    for source in ARCHIVES {
        let url = build_url(source, app_id, &lua_filename);
        match client
            .head(&url)
            .header("User-Agent", "SteamManifestDownloader")
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => return Ok(true),
            _ => continue,
        }
    }

    Ok(false)
}

pub async fn download_text_file(
    client: &Client,
    app_id: &str,
    filename: &str,
) -> Result<String, String> {
    let mut last_status = String::new();

    for source in ARCHIVES {
        let url = build_url(source, app_id, filename);
        match client
            .get(&url)
            .header("User-Agent", "SteamManifestDownloader")
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                return resp
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response: {}", e));
            }
            Ok(resp) => {
                last_status = format!("HTTP {}", resp.status());
            }
            Err(e) => {
                last_status = e.to_string();
            }
        }
    }

    Err(format!("File not found: {} ({})", filename, last_status))
}

async fn download_text_file_from_all(
    client: &Client,
    app_id: &str,
    filename: &str,
) -> Vec<String> {
    let mut results = Vec::new();

    for source in ARCHIVES {
        let url = build_url(source, app_id, filename);
        if let Ok(resp) = client
            .get(&url)
            .header("User-Agent", "SteamManifestDownloader")
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    results.push(text);
                }
            }
        }
    }

    results
}

pub async fn download_manifest_file(
    client: &Client,
    app_id: &str,
    depot_id: &str,
    manifest_id: &str,
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let filename = format!("{}_{}.manifest", depot_id, manifest_id);
    let mut last_error = String::from("No archives available");

    for source in ARCHIVES {
        let url = build_url(source, app_id, &filename);
        match client
            .get(&url)
            .header("User-Agent", "SteamManifestDownloader")
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let bytes = resp
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

                return Ok(output_path);
            }
            Ok(resp) => {
                last_error = format!("HTTP {}", resp.status());
            }
            Err(e) => {
                last_error = format!("Download failed: {}", e);
            }
        }
    }

    Err(format!("Manifest not found: {} ({})", filename, last_error))
}

// Returns (depot_id, manifest_gid) → size.
fn extract_depot_sizes(json: &serde_json::Value) -> HashMap<(String, String), u64> {
    let mut sizes = HashMap::new();

    let depot_obj = match json.get("depot").and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => return sizes,
    };

    for (depot_id, depot_value) in depot_obj {
        // "depot" also contains non-depot keys like "depotdeltapatches" / "baselanguages".
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

            // size == 7 is a placeholder meaning "unknown"; drop it.
            if size > 100 {
                sizes.insert((depot_id.clone(), gid), size);
            }
        }
    }

    sizes
}

pub async fn get_app_data(
    client: &Client,
    app_id: &str,
) -> Result<AppArchiveData, String> {
    let lua_filename = format!("{}.lua", app_id);
    let lua_content = download_text_file(client, app_id, &lua_filename).await?;

    let lua_result = crate::services::lua_parser::parse_lua_file(&lua_content)
        .map_err(|e| format!("Failed to parse {}.lua from Internet Archive: {}", app_id, e))?;

    // key.vdf values differ between archives; merge all of them so more depots get keys.
    let mut depot_keys = HashMap::new();
    for vdf_content in download_text_file_from_all(client, app_id, "key.vdf").await {
        let vdf_keys =
            crate::services::vdf_parser::parse_key_vdf(&vdf_content, Some("InternetArchive"));
        depot_keys.extend(vdf_keys);
    }

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
        Err(_) => HashMap::new(),
    };

    for depot in &lua_result.depots {
        if let Some(ref key) = depot.depot_key {
            depot_keys.insert(depot.depot_id.to_string(), key.clone());
        }
    }

    let manifests: Vec<ManifestInfo> = lua_result
        .depots
        .iter()
        .filter_map(|d| {
            d.manifest_id.as_ref().map(|mid| {
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