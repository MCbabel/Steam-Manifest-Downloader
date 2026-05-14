use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::services::depot_sources;
use crate::services::hubcap_api;

pub const HUBCAP_REPO_NAME: &str = "Hubcap (hubcapmanifest.com)";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoResult {
    pub repo: String,
    pub date: Option<String>,
    pub sha: Option<String>,
    #[serde(rename = "type")]
    pub source_type: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub repos: Vec<RepoResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestWithKey {
    pub depot_id: String,
    pub manifest_id: String,
    pub filename: String,
    pub depot_key: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoManifests {
    pub manifests: Vec<ManifestWithKey>,
    pub has_key_vdf: bool,
    pub key_vdf_filename: Option<String>,
    pub lua_filename: Option<String>,
    pub files: Vec<String>,
    pub depot_keys: HashMap<String, String>,
}

pub async fn search_repos(
    client: &Client,
    sources: &[String],
    app_id: &str,
    hubcap_api_key: &str,
    app_data_dir: &Path,
) -> Result<SearchResult, String> {
    let mut found = Vec::new();

    // Try hubcap first when an API key is configured.
    let has_key = !hubcap_api_key.is_empty();
    eprintln!("[Search] hubcap_api_key present: {}, length: {}", has_key, hubcap_api_key.len());
    if has_key {
        match hubcap_api::fetch_app_data(client, hubcap_api_key, app_data_dir, app_id).await {
            Ok(_) => {
                found.push(RepoResult {
                    repo: HUBCAP_REPO_NAME.to_string(),
                    date: None,
                    sha: None,
                    source_type: "hubcap".to_string(),
                    source: Some("Hubcap".to_string()),
                });
                return Ok(SearchResult { repos: found });
            }
            Err(e) => {
                // When hubcap is explicitly configured, surface its error to the
                // user rather than silently falling through to the "not found"
                // message from the depot-sources path.
                if !hubcap_api_key.is_empty() {
                    return Err(format!("Hubcap error: {}", e));
                }
                eprintln!("[Search] Hubcap lookup failed, falling back to repos: {}", e);
            }
        }
    }

    if sources.is_empty() {
        return Ok(SearchResult { repos: found });
    }

    match depot_sources::check_app_exists(client, sources, app_id).await {
        Ok(true) => {
            found.push(RepoResult {
                repo: "User-configured source".to_string(),
                date: None,
                sha: None,
                source_type: "remote".to_string(),
                source: Some("Configured manifest source".to_string()),
            });
        }
        Ok(false) => {}
        Err(e) => {
            eprintln!("[Search] manifest source check failed: {}", e);
        }
    }

    Ok(SearchResult { repos: found })
}

pub async fn get_repo_manifests(
    client: &Client,
    sources: &[String],
    app_id: &str,
    repo: &str,
    _sha: &str,
    hubcap_api_key: &str,
    app_data_dir: &Path,
) -> Result<RepoManifests, String> {
    let app_data = if repo == HUBCAP_REPO_NAME {
        hubcap_api::fetch_app_data(client, hubcap_api_key, app_data_dir, app_id).await?
    } else {
        depot_sources::get_app_data(client, sources, app_id).await?
    };

    let lua_filename = Some(format!("{}.lua", app_id));

    let mut files = Vec::new();
    if let Some(ref lua_file) = lua_filename {
        files.push(lua_file.clone());
    }
    if app_data.has_key_vdf {
        files.push("key.vdf".to_string());
    }

    let manifests_with_keys: Vec<ManifestWithKey> = app_data
        .manifests
        .into_iter()
        .map(|m| {
            let filename = format!("{}_{}.manifest", m.depot_id, m.manifest_id);
            files.push(filename.clone());
            ManifestWithKey {
                depot_id: m.depot_id,
                manifest_id: m.manifest_id,
                filename,
                depot_key: m.depot_key,
                size_bytes: m.size_bytes,
            }
        })
        .collect();

    let key_vdf_filename = if app_data.has_key_vdf {
        Some("key.vdf".to_string())
    } else {
        None
    };

    Ok(RepoManifests {
        manifests: manifests_with_keys,
        has_key_vdf: app_data.has_key_vdf,
        key_vdf_filename,
        lua_filename,
        files,
        depot_keys: app_data.depot_keys,
    })
}
