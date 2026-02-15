use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::services::github_api;
use crate::services::manifest_downloader;
use crate::services::vdf_parser;

/// Hardcoded list of GitHub repos to search for manifests.
pub const REPOS: &[&str] = &[
    "SteamAutoCracks/ManifestHub",
    "Flavor-Flavor/ManifestHub",
    "sean-who/ManifestHub",
    "NearlyTRex/SteamManifests",
    "PrintedWaste/GameManifests",
];

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
pub struct ManifestEntry {
    pub depot_id: String,
    pub manifest_id: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestWithKey {
    pub depot_id: String,
    pub manifest_id: String,
    pub filename: String,
    pub depot_key: Option<String>,
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

/// Search all repos for an App ID. Checks each repo in parallel for a branch matching the app_id.
pub async fn search_repos(
    client: &Client,
    app_id: &str,
    token: Option<&str>,
) -> Result<SearchResult, String> {
    let mut handles = Vec::new();

    for &repo in REPOS {
        let client = client.clone();
        let app_id = app_id.to_string();
        let token = token.map(String::from);

        handles.push(tokio::spawn(async move {
            let result = github_api::get_branch_info(
                &client,
                repo,
                &app_id,
                token.as_deref(),
            )
            .await;

            match result {
                Ok(branch_info) => {
                    if branch_info.rate_limited {
                        Some((None, true))
                    } else if branch_info.exists {
                        Some((
                            Some(RepoResult {
                                repo: repo.to_string(),
                                date: branch_info.last_updated,
                                sha: branch_info.sha,
                                source_type: "github".to_string(),
                                source: None,
                                download_url: None,
                                expires_at: None,
                            }),
                            false,
                        ))
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        }));
    }

    let mut found = Vec::new();
    let mut github_rate_limited = false;

    for handle in handles {
        if let Ok(Some((result, rate_limited))) = handle.await {
            if rate_limited {
                github_rate_limited = true;
            }
            if let Some(repo_result) = result {
                found.push(repo_result);
            }
        }
    }

    // Sort by date (newest first), items without dates go to the end
    found.sort_by(|a, b| {
        match (&a.date, &b.date) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(da), Some(db)) => db.cmp(da),
        }
    });

    Ok(SearchResult {
        repos: found,
        github_rate_limited,
    })
}

/// Get manifest file listing from a repo's branch using GitHub Tree API.
/// Parses tree entries to find `.manifest` files, `Key.vdf`/`key.vdf`, and `.lua` files.
/// If Key.vdf is found, downloads and parses it. If lua file is found, downloads and parses it.
pub async fn get_repo_manifests(
    client: &Client,
    app_id: &str,
    repo: &str,
    sha: &str,
    token: Option<&str>,
) -> Result<RepoManifests, String> {
    let tree_data = github_api::get_tree(client, repo, sha, token).await?;

    let tree = tree_data["tree"]
        .as_array()
        .ok_or("Missing tree array in GitHub response")?;

    let manifest_re = Regex::new(r"^(\d+)_(\d+)\.manifest$").unwrap();

    let mut manifests = Vec::new();
    let mut has_key_vdf = false;
    let mut key_vdf_filename: Option<String> = None;
    let mut lua_filename: Option<String> = None;
    let mut files = Vec::new();

    for item in tree {
        let item_type = item["type"].as_str().unwrap_or("");
        if item_type != "blob" {
            continue;
        }

        let path = item["path"].as_str().unwrap_or("");
        files.push(path.to_string());

        // Check for Key.vdf (case-insensitive)
        if path.to_lowercase() == "key.vdf" {
            has_key_vdf = true;
            key_vdf_filename = Some(path.to_string());
            continue;
        }

        // Check for .lua files
        if path.to_lowercase().ends_with(".lua") {
            lua_filename = Some(path.to_string());
        }

        // Parse manifest filenames like "1995891_3438272076824159257.manifest"
        if let Some(caps) = manifest_re.captures(path) {
            manifests.push(ManifestEntry {
                depot_id: caps[1].to_string(),
                manifest_id: caps[2].to_string(),
                filename: path.to_string(),
            });
        }
    }

    // Download and parse Key.vdf if present
    let mut depot_keys: HashMap<String, String> = HashMap::new();

    if has_key_vdf {
        if let Some(ref vdf_file) = key_vdf_filename {
            match manifest_downloader::download_key_vdf(
                client,
                app_id,
                repo,
                sha,
                Some(vdf_file.as_str()),
                token,
            )
            .await
            {
                Ok(vdf_content) => {
                    depot_keys = vdf_parser::parse_key_vdf(&vdf_content, Some(repo));
                }
                Err(e) => {
                    eprintln!("[MultiRepoSearch] Failed to download Key.vdf from {}: {}", repo, e);
                }
            }
        }
    }

    // Download and parse lua file if present
    if let Some(ref lua_file) = lua_filename {
        match manifest_downloader::download_repo_text_file(
            client,
            repo,
            app_id,
            lua_file,
            token,
        )
        .await
        {
            Ok(lua_content) => {
                let lua_result = crate::services::lua_parser::parse_lua_file(&lua_content);
                // Merge lua depot keys into our depot_keys map
                for depot in &lua_result.depots {
                    if let Some(ref key) = depot.depot_key {
                        depot_keys.insert(depot.depot_id.to_string(), key.clone());
                    }
                }
            }
            Err(e) => {
                eprintln!("[MultiRepoSearch] Failed to download lua file from {}: {}", repo, e);
            }
        }
    }

    // Combine manifests with depot keys
    let manifests_with_keys: Vec<ManifestWithKey> = manifests
        .into_iter()
        .map(|m| {
            let depot_key = depot_keys.get(&m.depot_id).cloned();
            ManifestWithKey {
                depot_id: m.depot_id,
                manifest_id: m.manifest_id,
                filename: m.filename,
                depot_key,
            }
        })
        .collect();

    Ok(RepoManifests {
        manifests: manifests_with_keys,
        has_key_vdf,
        key_vdf_filename,
        lua_filename,
        files,
        depot_keys,
    })
}
