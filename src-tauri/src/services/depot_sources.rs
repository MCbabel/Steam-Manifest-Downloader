use reqwest::Client;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
enum Backend {
    ArchiveOrg { base: String },
    GitHub { raw_base: String },
    Generic { base: String },
}

fn parse_backend(url: &str) -> Result<Backend, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("empty source URL".into());
    }

    if trimmed.contains("archive.org/view_archive.php") {
        return Ok(Backend::ArchiveOrg {
            base: trimmed.trim_end_matches('&').trim_end_matches('?').to_string(),
        });
    }

    if let Some(stripped) = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
    {
        let path = stripped.trim_end_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 2 {
            return Err("GitHub URL needs <user>/<repo>".into());
        }
        let user = parts[0];
        let repo = parts[1];
        let branch = if parts.len() >= 4 && parts[2] == "tree" {
            parts[3]
        } else {
            "main"
        };
        return Ok(Backend::GitHub {
            raw_base: format!("https://raw.githubusercontent.com/{}/{}/{}", user, repo, branch),
        });
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Ok(Backend::Generic {
            base: trimmed.trim_end_matches('/').to_string(),
        });
    }

    Err(format!("unsupported source URL: {}", trimmed))
}

fn build_url(backend: &Backend, app_id: &str, filename: &str) -> String {
    match backend {
        Backend::ArchiveOrg { base } => {
            let file_path = format!("branches/{}/{}", app_id, filename);
            let encoded = file_path.replace('/', "%2F");
            let sep = if base.contains('?') { '&' } else { '?' };
            format!("{}{}file={}", base, sep, encoded)
        }
        Backend::GitHub { raw_base } => {
            format!("{}/branches/{}/{}", raw_base, app_id, filename)
        }
        Backend::Generic { base } => {
            format!("{}/branches/{}/{}", base, app_id, filename)
        }
    }
}

fn parse_sources(sources: &[String]) -> Vec<Backend> {
    sources
        .iter()
        .filter_map(|s| match parse_backend(s) {
            Ok(b) => Some(b),
            Err(e) => {
                eprintln!("[DepotSources] skipping invalid source '{}': {}", s, e);
                None
            }
        })
        .collect()
}

pub async fn check_app_exists(
    client: &Client,
    sources: &[String],
    app_id: &str,
) -> Result<bool, String> {
    let backends = parse_sources(sources);
    if backends.is_empty() {
        return Ok(false);
    }
    let lua_filename = format!("{}.lua", app_id);

    for backend in &backends {
        let url = build_url(backend, app_id, &lua_filename);
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
    sources: &[String],
    app_id: &str,
    filename: &str,
) -> Result<String, String> {
    let backends = parse_sources(sources);
    if backends.is_empty() {
        return Err("no manifest sources configured".into());
    }
    let mut last_status = String::new();

    for backend in &backends {
        let url = build_url(backend, app_id, filename);
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
    sources: &[String],
    app_id: &str,
    filename: &str,
) -> Vec<String> {
    let backends = parse_sources(sources);
    let mut results = Vec::new();

    for backend in &backends {
        let url = build_url(backend, app_id, filename);
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
    sources: &[String],
    app_id: &str,
    depot_id: &str,
    manifest_id: &str,
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let backends = parse_sources(sources);
    if backends.is_empty() {
        return Err("no manifest sources configured".into());
    }
    let filename = format!("{}_{}.manifest", depot_id, manifest_id);
    let mut last_error = String::from("no source returned the file");

    for backend in &backends {
        let url = build_url(backend, app_id, &filename);
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

fn extract_depot_sizes(json: &serde_json::Value) -> HashMap<(String, String), u64> {
    let mut sizes = HashMap::new();

    let depot_obj = match json.get("depot").and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => return sizes,
    };

    for (depot_id, depot_value) in depot_obj {
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

            // size == 7 is the upstream placeholder for "unknown"
            if size > 100 {
                sizes.insert((depot_id.clone(), gid), size);
            }
        }
    }

    sizes
}

pub async fn get_app_data(
    client: &Client,
    sources: &[String],
    app_id: &str,
) -> Result<AppArchiveData, String> {
    let lua_filename = format!("{}.lua", app_id);
    let lua_content = download_text_file(client, sources, app_id, &lua_filename).await?;

    let lua_result = crate::services::lua_parser::parse_lua_file(&lua_content)
        .map_err(|e| format!("Failed to parse {}.lua: {}", app_id, e))?;

    let mut depot_keys = HashMap::new();
    for vdf_content in download_text_file_from_all(client, sources, app_id, "key.vdf").await {
        let vdf_keys = crate::services::vdf_parser::parse_key_vdf(&vdf_content, None);
        depot_keys.extend(vdf_keys);
    }

    let depot_sizes = match download_text_file(client, sources, app_id, &format!("{}.json", app_id)).await {
        Ok(json_content) => match serde_json::from_str::<serde_json::Value>(&json_content) {
            Ok(json) => extract_depot_sizes(&json),
            Err(e) => {
                eprintln!("[DepotSources] failed to parse {}.json: {}", app_id, e);
                HashMap::new()
            }
        },
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