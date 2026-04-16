use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::services::internet_archive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoResult {
    pub repo: String,
    pub date: Option<String>,
    pub sha: Option<String>,
    #[serde(rename = "type")]
    pub source_type: String,
    /// For alternative sources
    pub source: Option<String>,
    /// KernelOS download URL
    pub download_url: Option<String>,
    /// KernelOS expiry
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub repos: Vec<RepoResult>,
    pub github_rate_limited: bool,
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

/// Search Internet Archive for an App ID.
/// Returns a SearchResult with a single RepoResult if the app exists.
pub async fn search_repos(
    client: &Client,
    app_id: &str,
) -> Result<SearchResult, String> {
    let mut found = Vec::new();

    // Check Internet Archive
    match internet_archive::check_app_exists(client, app_id).await {
        Ok(true) => {
            found.push(RepoResult {
                repo: "Internet Archive".to_string(),
                date: None,
                sha: None,
                source_type: "archive".to_string(),
                source: Some("Internet Archive".to_string()),
                download_url: None,
                expires_at: None,
            });
        }
        Ok(false) => {}
        Err(e) => {
            eprintln!("[MultiRepoSearch] Internet Archive check failed: {}", e);
        }
    }

    Ok(SearchResult {
        repos: found,
        github_rate_limited: false,
    })
}

/// Get manifest file listing from the Internet Archive for an app.
/// Downloads and parses the .lua file and optional key.vdf to build the manifest list.
pub async fn get_repo_manifests(
    client: &Client,
    app_id: &str,
    _repo: &str,
    _sha: &str,
) -> Result<RepoManifests, String> {
    let app_data = internet_archive::get_app_data(client, app_id).await?;

    let lua_filename = Some(format!("{}.lua", app_id));

    // Build file list from manifests
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
