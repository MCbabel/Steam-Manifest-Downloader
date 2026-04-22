use tauri::command;
use crate::services::AppState;
use crate::services::multi_repo_search;
use crate::services::steam_store_api;

// Returns `{ repos: [...] }` — array shape preserved for the UI even though IA
// is currently the only source.
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