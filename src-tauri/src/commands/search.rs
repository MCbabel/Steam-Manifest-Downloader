use std::path::PathBuf;
use tauri::{command, AppHandle, Manager};
use crate::services::AppState;
use crate::services::depot_info::{self, DepotInfo};
use crate::services::multi_repo_search;
use crate::services::settings as settings_service;
use crate::services::steam_store_api;

fn app_data_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."))
}

async fn load_sources(app: &AppHandle) -> Vec<String> {
    settings_service::load_settings(&app_data_dir(app))
        .await
        .depot_sources
}

async fn load_hubcap_key(app: &AppHandle) -> String {
    settings_service::load_settings(&app_data_dir(app))
        .await
        .hubcap_api_key
}

#[command]
pub async fn search_repos(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    app_id: String,
) -> Result<serde_json::Value, String> {
    let sources = load_sources(&app).await;
    let hubcap_key = load_hubcap_key(&app).await;
    let dir = app_data_dir(&app);
    let result = multi_repo_search::search_repos(
        &state.http_client,
        &sources,
        &app_id,
        &hubcap_key,
        &dir,
    )
    .await?;

    serde_json::to_value(&result).map_err(|e| format!("Failed to serialize search result: {}", e))
}

#[command]
pub async fn get_repo_manifests(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    app_id: String,
    repo: String,
    sha: Option<String>,
) -> Result<serde_json::Value, String> {
    let effective_sha = sha.unwrap_or_default();
    let sources = load_sources(&app).await;
    let hubcap_key = load_hubcap_key(&app).await;
    let dir = app_data_dir(&app);

    let result = multi_repo_search::get_repo_manifests(
        &state.http_client,
        &sources,
        &app_id,
        &repo,
        &effective_sha,
        &hubcap_key,
        &dir,
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
pub async fn fetch_depot_metadata(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    app_id: String,
) -> Result<Vec<DepotInfo>, String> {
    let dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    depot_info::fetch_depot_info(&state.http_client, &dir, &app_id).await
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