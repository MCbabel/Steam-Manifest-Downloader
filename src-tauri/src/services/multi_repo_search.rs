use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::services::depot_sources;

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
) -> Result<SearchResult, String> {
    let mut found = Vec::new();

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
    _repo: &str,
    _sha: &str,
) -> Result<RepoManifests, String> {
    let app_data = depot_sources::get_app_data(client, sources, app_id).await?;

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