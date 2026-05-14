use std::path::{Path, PathBuf};
use tauri::{command, AppHandle, Manager};

use crate::services::emulator::{
    self, EmuSettings, Platform, ReleaseInfo, ReplaceResult, ScannedFile, Variant,
};
use crate::services::AppState;

fn app_data_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[command]
pub async fn emu_release_info(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<ReleaseInfo, String> {
    let dir = app_data_dir(&app);
    emulator::fetch_release_info(&state.http_client, &dir).await
}

#[command]
pub async fn emu_ensure_cached(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    platform: Platform,
) -> Result<ReleaseInfo, String> {
    let dir = app_data_dir(&app);
    let info = emulator::fetch_release_info(&state.http_client, &dir).await?;
    emulator::ensure_cached(&state.http_client, &info, platform).await?;
    let refreshed = emulator::fetch_release_info(&state.http_client, &dir).await?;
    Ok(refreshed)
}

#[command]
pub async fn emu_scan_game_dir(game_dir: String) -> Result<Vec<ScannedFile>, String> {
    let path = PathBuf::from(&game_dir);
    if !path.exists() {
        return Err(format!("Path not found: {}", game_dir));
    }
    Ok(emulator::scan_game_dir(&path))
}

#[command]
pub async fn emu_apply_replacement(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    targets: Vec<ScannedFile>,
    variant: Variant,
    app_id: String,
    installed_app_ids: Vec<String>,
    emu_settings: Option<EmuSettings>,
) -> Result<Vec<ReplaceResult>, String> {
    let dir = app_data_dir(&app);
    let info = emulator::fetch_release_info(&state.http_client, &dir).await?;

    let need_windows = targets.iter().any(|t| t.platform == Platform::Windows);
    let need_linux = targets.iter().any(|t| t.platform == Platform::Linux);
    if need_windows {
        emulator::ensure_cached(&state.http_client, &info, Platform::Windows).await?;
    }
    if need_linux {
        emulator::ensure_cached(&state.http_client, &info, Platform::Linux).await?;
    }
    let cache_root = PathBuf::from(&info.cache_root);

    let settings_ref = emu_settings.as_ref();
    let mut results = Vec::with_capacity(targets.len());
    for t in &targets {
        let path = Path::new(&t.path);
        let platform_cache = cache_root.join(t.platform.cache_subdir());
        let x64 = t.arch == "x64";
        results.push(emulator::apply_replacement(
            path,
            &platform_cache,
            variant,
            t.platform,
            x64,
            &app_id,
            &installed_app_ids,
            settings_ref,
        ));
    }
    Ok(results)
}

#[command]
pub async fn emu_read_emu_settings(target_path: String) -> Result<EmuSettings, String> {
    let path = PathBuf::from(&target_path);
    emulator::read_emu_settings_for_target(&path)
}

#[command]
pub async fn emu_write_emu_settings(
    target_path: String,
    settings: EmuSettings,
) -> Result<(), String> {
    let path = PathBuf::from(&target_path);
    emulator::write_emu_settings_for_target(&path, &settings)
}

#[command]
pub async fn emu_revert_replacement(targets: Vec<String>) -> Result<Vec<ReplaceResult>, String> {
    let mut results = Vec::with_capacity(targets.len());
    for target in targets {
        let path = PathBuf::from(&target);
        let mut r = ReplaceResult {
            path: target.clone(),
            backup_path: None,
            success: false,
            error: None,
        };
        match emulator::revert_replacement(&path) {
            Ok(()) => r.success = true,
            Err(e) => r.error = Some(e),
        }
        results.push(r);
    }
    Ok(results)
}

#[command]
pub async fn emu_launch_lobby_connect(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    game_dir: String,
    app_id: String,
    platform: Platform,
    x64: bool,
) -> Result<u32, String> {
    let data_dir = app_data_dir(&app);
    let info = emulator::fetch_release_info(&state.http_client, &data_dir).await?;
    emulator::ensure_cached(&state.http_client, &info, platform).await?;
    let platform_cache = PathBuf::from(&info.cache_root).join(platform.cache_subdir());
    let tool = emulator::lobby_connect_tool(&platform_cache, x64, platform)?;

    let game_path = PathBuf::from(&game_dir);
    if !game_path.exists() {
        return Err(format!("game_dir does not exist: {}", game_dir));
    }

    let child = std::process::Command::new(&tool)
        .current_dir(&game_path)
        .env("SteamAppId", &app_id)
        .env("SteamGameId", &app_id)
        .spawn()
        .map_err(|e| format!("spawn lobby_connect: {}", e))?;
    Ok(child.id())
}
