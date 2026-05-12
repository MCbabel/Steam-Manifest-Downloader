use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use tauri::{command, AppHandle, Manager};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use uuid::Uuid;

use crate::services::{AppState, JobInfo};
use crate::services::depot_runner::{self, DepotRunConfig, ProgressEvent, emit_progress};
use crate::services::depot_sources;
use crate::services::settings as settings_service;
use crate::services::manifest_hub_api;
use crate::services::steam_store_api;
use crate::services::lua_parser::DepotInfo;
use crate::services::depot_keys_generator;
use crate::services::history;

// Keep finished jobs in the map briefly so the UI can still read final status.
const JOB_RETENTION: std::time::Duration = std::time::Duration::from_secs(30 * 60);

#[derive(Debug, Deserialize)]
pub struct DownloadConfig {
    #[serde(rename = "mainAppId", alias = "app_id")]
    pub app_id: String,
    #[serde(rename = "gameName", alias = "game_name")]
    pub game_name: Option<String>,
    #[serde(rename = "selectedDepots", alias = "depots")]
    pub depots: Vec<DepotConfig>,
    #[allow(dead_code)] // Deserialized from frontend JSON but not read directly by backend
    pub mode: Option<String>,
    #[serde(rename = "keyVdfKeys", alias = "key_vdf_keys")]
    pub key_vdf_keys: Option<HashMap<String, String>>,
    #[serde(rename = "downloadDir", alias = "download_location")]
    pub download_location: Option<String>,
    #[serde(rename = "manifestHubApiKey")]
    pub manifest_hub_api_key: Option<String>,
    #[serde(rename = "headerImage", default)]
    pub header_image: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DepotConfig {
    #[serde(rename = "depotId", alias = "depot_id")]
    pub depot_id: String,
    #[serde(rename = "manifestId", alias = "manifest_id")]
    pub manifest_id: String,
    #[serde(rename = "customManifestId", alias = "custom_manifest_id")]
    pub custom_manifest_id: Option<String>,
    #[serde(rename = "depotKey", alias = "depot_key")]
    pub depot_key: Option<String>,
    #[serde(rename = "uploadedManifestPath")]
    pub uploaded_manifest_path: Option<String>,
}

// Trust-boundary validation: pipeline assumes all IDs parse cleanly.
fn validate_download_ids(config: &DownloadConfig) -> Result<(), String> {
    config
        .app_id
        .parse::<u64>()
        .map_err(|_| format!("Invalid App ID '{}': expected a numeric Steam App ID", config.app_id))?;

    for depot in &config.depots {
        depot.depot_id.parse::<u64>().map_err(|_| {
            format!(
                "Invalid depot ID '{}': expected a numeric Steam depot ID",
                depot.depot_id
            )
        })?;
    }

    Ok(())
}

#[command]
pub async fn start_download(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    config: DownloadConfig,
) -> Result<serde_json::Value, String> {
    validate_download_ids(&config)?;

    let job_id = Uuid::new_v4().to_string();

    let base_dir = resolve_download_dir(config.download_location.as_deref())
        .unwrap_or_else(|| {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join("Documents").join("SteamDownloads")
        });

    tokio::fs::create_dir_all(&base_dir)
        .await
        .map_err(|e| format!("Cannot create download directory: {}", e))?;

    let mut folder_name = config.app_id.clone();
    let mut game_name = config.game_name.clone();
    let mut header_image: Option<String> = config.header_image.clone();

    if game_name.is_none() {
        match steam_store_api::get_game_info(
            &state.http_client,
            &state.steam_cache,
            &config.app_id,
        ).await {
            Ok(Some(info)) => {
                game_name = info.name.clone();
                header_image = info.header_image.clone();
                if let Some(ref name) = info.name {
                    let sanitized = steam_store_api::sanitize_game_name(name);
                    if !sanitized.is_empty() {
                        folder_name = format!("{} - {}", config.app_id, sanitized);
                    }
                }
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!(
                    "[Download] Steam Store lookup failed for app {}: {}. Using App ID as folder name.",
                    config.app_id, e
                );
            }
        }
    } else if let Some(ref name) = game_name {
        let sanitized = steam_store_api::sanitize_game_name(name);
        if !sanitized.is_empty() {
            folder_name = format!("{} - {}", config.app_id, sanitized);
        }
    }

    let download_dir = base_dir.join(&folder_name);

    {
        let mut jobs = state.active_jobs.lock().await;
        jobs.insert(
            job_id.clone(),
            JobInfo {
                status: "running".to_string(),
                child_pid: None,
                download_dir: Some(download_dir.to_string_lossy().to_string()),
                #[cfg(target_os = "windows")]
                job_object: None,
            },
        );
    }

    let response = serde_json::json!({
        "jobId": job_id,
        "downloadDir": download_dir.to_string_lossy(),
        "folderName": folder_name,
    });

    let job_id_clone = job_id.clone();
    let app_clone = app.clone();
    let http_client = state.http_client.clone();
    let active_jobs = state.active_jobs.clone();
    let steam_cache = state.steam_cache.clone();
    let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let download_dir_for_history = download_dir.to_string_lossy().to_string();
    tokio::spawn(async move {
        let started_at = chrono::Utc::now();

        let state_ref = AppState {
            app_handle: app_clone.clone(),
            active_jobs: active_jobs.clone(),
            http_client: http_client.clone(),
            steam_cache: steam_cache.clone(),
            telemetry: None,
        };

        let result = run_download_pipeline(
            &app_clone,
            &state_ref,
            &job_id_clone,
            &config,
            &base_dir,
            &folder_name,
            game_name.as_deref(),
            header_image.as_deref(),
            &app_data_dir,
        )
        .await;

        let is_cancelled = {
            let jobs = active_jobs.lock().await;
            jobs.get(&job_id_clone)
                .map(|j| j.status == "cancelled")
                .unwrap_or(false)
        };

        match result {
            Ok(_) => {
                if is_cancelled {
                    let entry = history::HistoryEntry {
                        id: Uuid::new_v4().to_string(),
                        app_id: config.app_id.clone(),
                        game_name: game_name.clone(),
                        header_image: header_image.clone(),
                        depot_count: config.depots.len(),
                        depots_downloaded: 0,
                        status: "cancelled".to_string(),
                        download_dir: download_dir_for_history.clone(),
                        started_at: started_at.to_rfc3339(),
                        completed_at: Some(chrono::Utc::now().to_rfc3339()),
                        source_repo: None,
                        depot_ids: config.depots.iter().map(|d| d.depot_id.clone()).collect(),
                    };
                    if let Err(err) = history::add_entry(&app_data_dir, entry).await {
                        eprintln!("[Download] Failed to record cancelled job in history: {}", err);
                    }
                }
            }
            Err(e) => {
                if !is_cancelled {
                    let mut event = ProgressEvent::new("error", &job_id_clone);
                    event.message = Some(format!("Unexpected error: {}", e));
                    emit_progress(&app_clone, &event);
                }

                let entry = history::HistoryEntry {
                    id: Uuid::new_v4().to_string(),
                    app_id: config.app_id.clone(),
                    game_name: game_name.clone(),
                    header_image: header_image.clone(),
                    depot_count: config.depots.len(),
                    depots_downloaded: 0,
                    status: if is_cancelled { "cancelled" } else { "failed" }.to_string(),
                    download_dir: download_dir_for_history.clone(),
                    started_at: started_at.to_rfc3339(),
                    completed_at: Some(chrono::Utc::now().to_rfc3339()),
                    source_repo: None,
                    depot_ids: config.depots.iter().map(|d| d.depot_id.clone()).collect(),
                };
                if let Err(err) = history::add_entry(&app_data_dir, entry).await {
                    eprintln!("[Download] Failed to record failed job in history: {}", err);
                }
            }
        }

        let active_jobs_cleanup = active_jobs.clone();
        let job_id_cleanup = job_id_clone.clone();
        tokio::spawn(async move {
            tokio::time::sleep(JOB_RETENTION).await;
            let mut jobs = active_jobs_cleanup.lock().await;
            jobs.remove(&job_id_cleanup);
        });
    });

    Ok(response)
}

async fn run_download_pipeline(
    app: &AppHandle,
    state: &AppState,
    job_id: &str,
    config: &DownloadConfig,
    base_dir: &Path,
    folder_name: &str,
    _game_name: Option<&str>,
    _header_image: Option<&str>,
    app_data_dir: &Path,
) -> Result<(), String> {
    let _started_at = chrono::Utc::now();
    let work_dir = base_dir.join(folder_name);
    let depot_sources_list = settings_service::load_settings(app_data_dir)
        .await
        .depot_sources;

    tokio::fs::create_dir_all(&work_dir)
        .await
        .map_err(|e| format!("Failed to create download directory: {}", e))?;

    if let Some(disk_info) = get_disk_space_info(base_dir) {
        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("disk_space".to_string());
        event.free_gb = Some(disk_info.0);
        event.drive = Some(disk_info.1);
        emit_progress(app, &event);
    }

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let uploaded_depots: Vec<&DepotConfig> = config.depots.iter().filter(|d| d.uploaded_manifest_path.is_some()).collect();
    let custom_depots: Vec<&DepotConfig> = config.depots.iter().filter(|d| d.uploaded_manifest_path.is_none() && d.custom_manifest_id.is_some()).collect();
    let standard_depots: Vec<&DepotConfig> = config.depots.iter().filter(|d| d.uploaded_manifest_path.is_none() && d.custom_manifest_id.is_none()).collect();

    if !standard_depots.is_empty() {
        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("checking_branch".to_string());
        event.app_id = Some(config.app_id.clone());
        emit_progress(app, &event);

        if check_cancelled(state, job_id).await {
            return Ok(());
        }

        if depot_sources_list.is_empty() {
            let mut event = ProgressEvent::new("error", job_id);
            event.message = Some(
                "No manifest sources configured. Add one in Settings → Advanced Settings → Manifest Sources."
                    .to_string(),
            );
            emit_progress(app, &event);
            return Ok(());
        }

        match depot_sources::check_app_exists(&state.http_client, &depot_sources_list, &config.app_id).await {
            Ok(true) => {
                let mut event = ProgressEvent::new("status", job_id);
                event.step = Some("branch_found".to_string());
                event.app_id = Some(config.app_id.clone());
                event.last_updated = Some("Source: configured manifest source".to_string());
                emit_progress(app, &event);
            }
            Ok(false) => {
                let mut event = ProgressEvent::new("error", job_id);
                event.message = Some(format!("App {} not found in any configured manifest source", config.app_id));
                emit_progress(app, &event);
                return Ok(());
            }
            Err(e) => {
                let mut event = ProgressEvent::new("error", job_id);
                event.message = Some(format!("Manifest source lookup failed: {}", e));
                emit_progress(app, &event);
                return Ok(());
            }
        }
    }

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let total_manifests = config.depots.len();
    let mut event = ProgressEvent::new("status", job_id);
    event.step = Some("downloading_manifests".to_string());
    event.total = Some(total_manifests);
    emit_progress(app, &event);

    let mut manifest_results: Vec<(String, bool)> = Vec::new();

    for depot in &uploaded_depots {
        if let Some(ref uploaded_path) = depot.uploaded_manifest_path {
            let manifest_id = depot.custom_manifest_id.as_deref().unwrap_or(&depot.manifest_id);
            let filename = format!("{}_{}.manifest", depot.depot_id, manifest_id);
            let dest_path = work_dir.join(&filename);

            match tokio::fs::copy(uploaded_path, &dest_path).await {
                Ok(_) => {
                    if let Err(err) = tokio::fs::remove_file(uploaded_path).await {
                        eprintln!(
                            "[Download] Failed to remove temp upload '{}': {}",
                            uploaded_path, err
                        );
                    }
                    let mut event = ProgressEvent::new("status", job_id);
                    event.step = Some("downloading_manifest".to_string());
                    event.depot_id = Some(depot.depot_id.clone());
                    event.manifest_id = Some(manifest_id.to_string());
                    event.filename = Some(filename);
                    event.message = Some("Using uploaded manifest file".to_string());
                    emit_progress(app, &event);
                    manifest_results.push((depot.depot_id.clone(), true));
                }
                Err(e) => {
                    let mut event = ProgressEvent::new("error", job_id);
                    event.message = Some(format!("Failed to use uploaded manifest for depot {}: {}", depot.depot_id, e));
                    emit_progress(app, &event);
                    manifest_results.push((depot.depot_id.clone(), false));
                }
            }
        }
    }

    for depot in &standard_depots {
        if check_cancelled(state, job_id).await {
            return Ok(());
        }

        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("downloading_manifest".to_string());
        event.depot_id = Some(depot.depot_id.clone());
        event.manifest_id = Some(depot.manifest_id.clone());
        emit_progress(app, &event);

        match depot_sources::download_manifest_file(
            &state.http_client,
            &depot_sources_list,
            &config.app_id,
            &depot.depot_id,
            &depot.manifest_id,
            &work_dir,
        )
        .await
        {
            Ok(_) => {
                manifest_results.push((depot.depot_id.clone(), true));
            }
            Err(e) => {
                let mut event = ProgressEvent::new("error", job_id);
                event.message = Some(format!("Failed to download manifest for depot {}: {}", depot.depot_id, e));
                emit_progress(app, &event);
                manifest_results.push((depot.depot_id.clone(), false));
            }
        }
    }

    for depot in &custom_depots {
        if check_cancelled(state, job_id).await {
            return Ok(());
        }

        let manifest_id = depot.custom_manifest_id.as_deref().unwrap_or(&depot.manifest_id);

        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("downloading_manifest_hub".to_string());
        event.depot_id = Some(depot.depot_id.clone());
        event.manifest_id = Some(manifest_id.to_string());
        emit_progress(app, &event);

        let api_key = config.manifest_hub_api_key.as_deref().unwrap_or_default();

        match manifest_hub_api::download_from_manifest_hub(
            &state.http_client,
            &config.app_id,
            &depot.depot_id,
            manifest_id,
            &work_dir,
            api_key,
        )
        .await
        {
            Ok(_) => {
                manifest_results.push((depot.depot_id.clone(), true));
            }
            Err(e) => {
                let mut event = ProgressEvent::new("error", job_id);
                event.message = Some(format!("Failed to download custom manifest for depot {}: {}", depot.depot_id, e));
                emit_progress(app, &event);
                manifest_results.push((depot.depot_id.clone(), false));
            }
        }
    }

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let success_count = manifest_results.iter().filter(|(_, s)| *s).count();
    if success_count == 0 && !manifest_results.is_empty() {
        let error_msg = "All manifest downloads failed".to_string();
        let mut event = ProgressEvent::new("error", job_id);
        event.message = Some(error_msg.clone());
        emit_progress(app, &event);

        let entry = history::HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            app_id: config.app_id.clone(),
            game_name: _game_name.map(|s| s.to_string()),
            header_image: _header_image.map(|s| s.to_string()),
            depot_count: config.depots.len(),
            depots_downloaded: 0,
            status: "failed".to_string(),
            download_dir: work_dir.to_string_lossy().to_string(),
            started_at: _started_at.to_rfc3339(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            source_repo: None,
            depot_ids: config.depots.iter().map(|d| d.depot_id.clone()).collect(),
        };
        if let Err(err) = history::add_entry(app_data_dir, entry).await {
            eprintln!("[Download] Failed to record failed download in history: {}", err);
        }

        return Ok(());
    }

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let mut event = ProgressEvent::new("status", job_id);
    event.step = Some("generating_keys".to_string());
    emit_progress(app, &event);

    let mut depot_infos: Vec<DepotInfo> = config
        .depots
        .iter()
        .map(|d| {
            let mut key = d.depot_key.clone();

            if key.is_none() {
                if let Some(ref kvk) = config.key_vdf_keys {
                    key = kvk.get(&d.depot_id).cloned();
                }
            }

            DepotInfo {
                depot_id: d.depot_id.parse().expect("depot IDs are validated at entry"),
                depot_key: key,
                manifest_id: Some(d.custom_manifest_id.as_deref().unwrap_or(&d.manifest_id).to_string()),
            }
        })
        .collect();

    if depot_infos.iter().any(|d| d.depot_key.is_none()) {
        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("downloading_keyvdf".to_string());
        emit_progress(app, &event);

        match depot_sources::download_text_file(
            &state.http_client,
            &depot_sources_list,
            &config.app_id,
            "key.vdf",
        )
        .await
        {
            Ok(vdf_content) => {
                let vdf_keys = crate::services::vdf_parser::parse_key_vdf(
                    &vdf_content,
                    None,
                );
                for depot in &mut depot_infos {
                    if depot.depot_key.is_none() {
                        if let Some(key) = vdf_keys.get(&depot.depot_id.to_string()) {
                            depot.depot_key = Some(key.clone());
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[Download] Key.vdf download/parse skipped: {}", e);
            }
        }
    }

    let keys_result = depot_keys_generator::generate_depot_keys(
        config.app_id.parse().expect("app_id is validated at entry"),
        &depot_infos,
        Some(folder_name),
        base_dir,
    )
    .await?;

    let mut event = ProgressEvent::new("status", job_id);
    event.step = Some("keys_generated".to_string());
    event.depot_count = Some(keys_result.depot_count);
    emit_progress(app, &event);

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let exe_path = depot_runner::get_exe_path_async().await?;

    let successful_depot_ids: Vec<String> = manifest_results
        .iter()
        .filter(|(_, s)| *s)
        .map(|(id, _)| id.clone())
        .collect();

    let run_depots: Vec<DepotRunConfig> = config
        .depots
        .iter()
        .filter(|d| successful_depot_ids.contains(&d.depot_id))
        .map(|d| DepotRunConfig {
            depot_id: d.depot_id.clone(),
            manifest_id: d.custom_manifest_id.as_deref().unwrap_or(&d.manifest_id).to_string(),
        })
        .collect();

    let mut event = ProgressEvent::new("status", job_id);
    event.step = Some("starting_downloader".to_string());
    event.total = Some(run_depots.len());
    emit_progress(app, &event);

    let settings = crate::services::settings::load_settings(app_data_dir).await;
    let extra_args = if settings.dd_extra_args.is_empty() {
        vec![
            "-max-downloads".to_string(),
            "8".to_string(),
            "-verify-all".to_string(),
        ]
    } else {
        settings.dd_extra_args.clone()
    };

    let download_results = depot_runner::run_all_depots(
        app,
        &exe_path,
        &config.app_id,
        &run_depots,
        &work_dir,
        &extra_args,
        job_id,
        state,
    )
    .await?;

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let dl_success_count = download_results.iter().filter(|r| r["success"].as_bool().unwrap_or(false)).count();
    let mut event = ProgressEvent::new("complete", job_id);
    event.message = Some(format!(
        "Download complete. {}/{} depots downloaded successfully.",
        dl_success_count,
        run_depots.len()
    ));
    event.results = Some(serde_json::Value::Array(download_results));
    emit_progress(app, &event);

    let status = if dl_success_count == run_depots.len() {
        "complete"
    } else if dl_success_count > 0 {
        "partial"
    } else {
        "failed"
    };
    let entry = history::HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        app_id: config.app_id.clone(),
        game_name: _game_name.map(|s| s.to_string()),
        header_image: _header_image.map(|s| s.to_string()),
        depot_count: run_depots.len(),
        depots_downloaded: dl_success_count,
        status: status.to_string(),
        download_dir: work_dir.to_string_lossy().to_string(),
        started_at: _started_at.to_rfc3339(),
        completed_at: Some(chrono::Utc::now().to_rfc3339()),
        source_repo: None,
        depot_ids: run_depots.iter().map(|d| d.depot_id.clone()).collect(),
    };
    if let Err(err) = history::add_entry(app_data_dir, entry).await {
        eprintln!("[Download] Failed to record completed job in history: {}", err);
    }

    {
        let mut jobs = state.active_jobs.lock().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = "complete".to_string();
        }
    }

    Ok(())
}

#[command]
pub async fn cancel_download(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    job_id: String,
) -> Result<(), String> {
    let download_dir = {
        let jobs = state.active_jobs.lock().await;
        if !jobs.contains_key(&job_id) {
            return Err("Job not found".to_string());
        }
        jobs.get(&job_id).and_then(|j| j.download_dir.clone())
    };

    depot_runner::kill_job(&state, &job_id).await;

    let mut event = ProgressEvent::new("cancelled", &job_id);
    event.message = Some("Download cancelled and files are being cleaned up.".to_string());
    emit_progress(&app, &event);

    if let Some(dir) = download_dir {
        let dir_path = std::path::PathBuf::from(&dir);
        if dir_path.exists() {
            // Give the child a moment to release file handles before rm -rf.
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                for attempt in 0..3 {
                    match tokio::fs::remove_dir_all(&dir_path).await {
                        Ok(_) => {
                            eprintln!("[Cancel] Cleaned up download directory: {:?}", dir_path);
                            break;
                        }
                        Err(e) => {
                            eprintln!("[Cancel] Attempt {} to delete {:?} failed: {}", attempt + 1, dir_path, e);
                            if attempt < 2 {
                                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                            }
                        }
                    }
                }
            });
        }
    }

    Ok(())
}

async fn check_cancelled(state: &AppState, job_id: &str) -> bool {
    let jobs = state.active_jobs.lock().await;
    jobs.get(job_id)
        .map(|j| j.status == "cancelled")
        .unwrap_or(false)
}

fn resolve_download_dir(dir_path: Option<&str>) -> Option<PathBuf> {
    let path_str = dir_path?.trim();
    if path_str.is_empty() {
        return None;
    }

    let resolved = PathBuf::from(path_str);
    if !resolved.is_absolute() {
        return None;
    }
    if resolved.to_string_lossy().len() < 3 {
        return None;
    }

    Some(resolved)
}

#[cfg(target_os = "windows")]
fn get_disk_space_info(path: &Path) -> Option<(f64, String)> {
    let path_str = path.to_string_lossy();
    if path_str.len() < 2 {
        return None;
    }

    let drive_letter = path_str.chars().next()?;
    let drive = format!("{}:", drive_letter);

    let mut cmd = std::process::Command::new("powershell");
    cmd.args([
            "-NoProfile",
            "-Command",
            &format!("(Get-PSDrive {}).Free", drive_letter),
        ]);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let output = cmd.output().ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let free_bytes: u64 = stdout.trim().parse().ok()?;
    let free_gb = (free_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
    let free_gb = (free_gb * 100.0).round() / 100.0;

    Some((free_gb, drive))
}

#[cfg(target_os = "linux")]
fn get_disk_space_info(path: &Path) -> Option<(f64, String)> {
    use std::ffi::CString;

    let path_str = path.to_string_lossy();
    let c_path = CString::new(path_str.as_ref()).ok()?;

    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        let result = libc::statvfs(c_path.as_ptr(), &mut stat);
        if result != 0 {
            return None;
        }

        let free = (stat.f_bavail as u64) * (stat.f_frsize as u64);
        let free_gb = (free as f64) / (1024.0 * 1024.0 * 1024.0);
        let free_gb = (free_gb * 100.0).round() / 100.0;

        Some((free_gb, path_str.to_string()))
    }
}
