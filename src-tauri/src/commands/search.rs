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
) -> Result<serde_json::Value, String> {
    let result = multi_repo_search::search_repos(
        &state.http_client,
        &app_id,
    )
    .await?;

    serde_json::to_value(&result).map_err(|e| format!("Failed to serialize search result: {}", e))
}

/// Get manifest file listing from a repo / Internet Archive.
/// Returns manifests list with depot keys.
#[command]
pub async fn get_repo_manifests(
    state: tauri::State<'_, AppState>,
    app_id: String,
    repo: String,
    sha: Option<String>,
) -> Result<serde_json::Value, String> {
    let effective_sha = sha.unwrap_or_default();

    let result = multi_repo_search::get_repo_manifests(
        &state.http_client,
        &app_id,
        &repo,
        &effective_sha,
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

/// Search Steam Store for games by name.
/// Returns a list of matching games with appId, name, and image.
#[command]
pub async fn search_steam_games(
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<serde_json::Value, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(serde_json::json!([]));
    }

    let response = state
        .http_client
        .get("https://store.steampowered.com/api/storesearch/")
        .query(&[("term", query.as_str()), ("l", "english"), ("cc", "US")])
        .send()
        .await
        .map_err(|e| format!("[SteamSearch] Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "[SteamSearch] API returned status {}",
            response.status()
        ));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("[SteamSearch] Failed to parse JSON: {}", e))?;

    let items = data
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let results: Vec<serde_json::Value> = items
        .iter()
        .take(10)
        .filter_map(|item| {
            let id = item.get("id")?.as_u64()?;
            let name = item.get("name")?.as_str()?;
            let image = item
                .get("tiny_image")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Some(serde_json::json!({
                "appId": id,
                "name": name,
                "image": image
            }))
        })
        .collect();

    Ok(serde_json::json!(results))
}
