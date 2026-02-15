use tauri::command;
use crate::services::AppState;
use crate::services::multi_repo_search;
use crate::services::alternative_sources;
use crate::services::steam_store_api;

/// Search all known repos for an App ID.
/// Returns { repos: [...], githubRateLimited: bool }
#[command]
pub async fn search_repos(
    state: tauri::State<'_, AppState>,
    app_id: String,
    github_token: Option<String>,
) -> Result<serde_json::Value, String> {
    let result = multi_repo_search::search_repos(
        &state.http_client,
        &app_id,
        github_token.as_deref(),
    )
    .await?;

    serde_json::to_value(&result).map_err(|e| format!("Failed to serialize search result: {}", e))
}

/// Get manifest file listing from a repo's branch.
/// Returns manifests list with depot keys.
#[command]
pub async fn get_repo_manifests(
    state: tauri::State<'_, AppState>,
    app_id: String,
    repo: String,
    sha: Option<String>,
    github_token: Option<String>,
) -> Result<serde_json::Value, String> {
    // If no SHA provided, we need to look up the branch first
    let effective_sha = match sha {
        Some(s) if !s.is_empty() => s,
        _ => {
            // Use app_id as branch name to get the SHA
            let branch_info = crate::services::github_api::get_branch_info(
                &state.http_client,
                &repo,
                &app_id,
                github_token.as_deref(),
            )
            .await?;

            branch_info
                .sha
                .ok_or_else(|| format!("Could not determine SHA for branch {} in {}", app_id, repo))?
        }
    };

    let result = multi_repo_search::get_repo_manifests(
        &state.http_client,
        &app_id,
        &repo,
        &effective_sha,
        github_token.as_deref(),
    )
    .await?;

    serde_json::to_value(&result).map_err(|e| format!("Failed to serialize manifests: {}", e))
}

/// Search alternative sources (kernelos or printedwaste).
#[command]
pub async fn search_alternative(
    state: tauri::State<'_, AppState>,
    app_id: String,
    source: String,
) -> Result<serde_json::Value, String> {
    match source.to_lowercase().as_str() {
        "printedwaste" => {
            let result = alternative_sources::download_from_printed_waste(
                &state.http_client,
                &app_id,
            )
            .await?;
            serde_json::to_value(&result)
                .map_err(|e| format!("Failed to serialize PrintedWaste result: {}", e))
        }
        "kernelos" => {
            // Use a temp directory for KernelOS extraction
            let temp_dir = std::env::temp_dir().join("steam_manifest_downloader");
            let result = alternative_sources::download_from_kernel_os(
                &state.http_client,
                &app_id,
                &temp_dir,
            )
            .await?;
            serde_json::to_value(&result)
                .map_err(|e| format!("Failed to serialize KernelOS result: {}", e))
        }
        _ => Err(format!("Unknown alternative source: {}. Use 'kernelos' or 'printedwaste'.", source)),
    }
}

/// Get Steam Store app info (name, header image, etc.).
#[command]
pub async fn get_steam_app_info(
    state: tauri::State<'_, AppState>,
    app_id: String,
) -> Result<serde_json::Value, String> {
    let info = steam_store_api::get_game_info(
        &state.http_client,
        &state.steam_cache,
        &app_id,
    )
    .await?;

    match info {
        Some(game_info) => serde_json::to_value(&game_info)
            .map_err(|e| format!("Failed to serialize game info: {}", e)),
        None => Ok(serde_json::Value::Null),
    }
}
