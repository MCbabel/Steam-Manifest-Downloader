use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use tauri::{command, AppHandle, Manager};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use uuid::Uuid;

use crate::services::{AppState, JobInfo};
use crate::services::depot_runner::{self, DepotRunConfig, ProgressEvent, emit_progress};
use crate::services::manifest_downloader;
use crate::services::manifest_hub_api;
use crate::services::steam_store_api;
use crate::services::vdf_parser;
use crate::services::lua_parser::DepotInfo;
use crate::services::depot_keys_generator;

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
    pub repo: Option<String>,
    pub sha: Option<String>,
    #[serde(rename = "githubToken", alias = "github_token")]
    pub github_token: Option<String>,
    #[serde(rename = "keyVdfKeys", alias = "key_vdf_keys")]
    pub key_vdf_keys: Option<HashMap<String, String>>,
    #[serde(rename = "downloadDir", alias = "download_location")]
    pub download_location: Option<String>,
    #[serde(rename = "manifestHubApiKey")]
    pub manifest_hub_api_key: Option<String>,
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

/// Start a download job. Returns { jobId, downloadDir } immediately,
/// then runs the download pipeline asynchronously emitting progress events.
#[command]
pub async fn start_download(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    config: DownloadConfig,
) -> Result<serde_json::Value, String> {
    let job_id = Uuid::new_v4().to_string();

    // Determine base download directory
    let base_dir = resolve_download_dir(config.download_location.as_deref())
        .unwrap_or_else(|| {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join("Documents").join("SteamDownloads")
        });

    // Create base dir
    tokio::fs::create_dir_all(&base_dir)
        .await
        .map_err(|e| format!("Cannot create download directory: {}", e))?;

    // Fetch game info for folder naming
    let mut folder_name = config.app_id.clone();
    let mut game_name = config.game_name.clone();
    let mut header_image: Option<String> = None;

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
            _ => {}
        }
    } else if let Some(ref name) = game_name {
        let sanitized = steam_store_api::sanitize_game_name(name);
        if !sanitized.is_empty() {
            folder_name = format!("{} - {}", config.app_id, sanitized);
        }
    }

    let download_dir = base_dir.join(&folder_name);

    // Register job
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

    // Clone what we need for the async task
    let job_id_clone = job_id.clone();
    let app_clone = app.clone();
    let http_client = state.http_client.clone();
    let active_jobs = state.active_jobs.clone();
    let steam_cache = state.steam_cache.clone();
    let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Spawn the download pipeline
    tokio::spawn(async move {
        let state_ref = AppState {
            app_handle: app_clone.clone(),
            active_jobs: active_jobs.clone(),
            http_client: http_client.clone(),
            steam_cache: steam_cache.clone(),
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

        match result {
            Ok(_) => {}
            Err(e) => {
                // Check if cancelled
                let is_cancelled = {
                    let jobs = active_jobs.lock().await;
                    jobs.get(&job_id_clone)
                        .map(|j| j.status == "cancelled")
                        .unwrap_or(false)
                };

                if !is_cancelled {
                    let mut event = ProgressEvent::new("error", &job_id_clone);
                    event.message = Some(format!("Unexpected error: {}", e));
                    emit_progress(&app_clone, &event);
                }
            }
        }

        // Schedule cleanup after 30 min
        let active_jobs_cleanup = active_jobs.clone();
        let job_id_cleanup = job_id_clone.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(30 * 60)).await;
            let mut jobs = active_jobs_cleanup.lock().await;
            jobs.remove(&job_id_cleanup);
        });
    });

    Ok(response)
}

/// The main download pipeline logic.
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

    // Create work directory
    tokio::fs::create_dir_all(&work_dir)
        .await
        .map_err(|e| format!("Failed to create download directory: {}", e))?;

    // Check for disk space
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

    // Categorize depots
    let uploaded_depots: Vec<&DepotConfig> = config.depots.iter().filter(|d| d.uploaded_manifest_path.is_some()).collect();
    let custom_depots: Vec<&DepotConfig> = config.depots.iter().filter(|d| d.uploaded_manifest_path.is_none() && d.custom_manifest_id.is_some()).collect();
    let standard_depots: Vec<&DepotConfig> = config.depots.iter().filter(|d| d.uploaded_manifest_path.is_none() && d.custom_manifest_id.is_none()).collect();

    // Step 1: Branch check (only for standard depots when no repo provided)
    if !standard_depots.is_empty() && config.repo.is_none() {
        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("checking_branch".to_string());
        event.app_id = Some(config.app_id.clone());
        emit_progress(app, &event);

        if check_cancelled(state, job_id).await {
            return Ok(());
        }

        let branch_result = crate::services::github_api::check_branch(
            &state.http_client,
            &config.app_id,
            config.github_token.as_deref(),
        )
        .await?;

        if !branch_result.exists {
            let error_msg = branch_result.error.unwrap_or_else(|| format!("Branch not found for AppID {}", config.app_id));
            let mut event = ProgressEvent::new("error", job_id);
            event.message = Some(error_msg.clone());
            emit_progress(app, &event);
            return Ok(());
        }

        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("branch_found".to_string());
        event.app_id = Some(config.app_id.clone());
        event.last_updated = branch_result.last_updated;
        emit_progress(app, &event);
    } else if config.repo.is_some() {
        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("branch_found".to_string());
        event.app_id = Some(config.app_id.clone());
        event.last_updated = Some(format!("Using repo: {}", config.repo.as_deref().unwrap_or("")));
        emit_progress(app, &event);
    }

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    // Step 2: Download manifest files
    let total_manifests = config.depots.len();
    let mut event = ProgressEvent::new("status", job_id);
    event.step = Some("downloading_manifests".to_string());
    event.total = Some(total_manifests);
    emit_progress(app, &event);

    let mut manifest_results: Vec<(String, bool)> = Vec::new(); // (depot_id, success)

    // Handle uploaded manifests - copy to work dir
    for depot in &uploaded_depots {
        if let Some(ref uploaded_path) = depot.uploaded_manifest_path {
            let manifest_id = depot.custom_manifest_id.as_deref().unwrap_or(&depot.manifest_id);
            let filename = format!("{}_{}.manifest", depot.depot_id, manifest_id);
            let dest_path = work_dir.join(&filename);

            match tokio::fs::copy(uploaded_path, &dest_path).await {
                Ok(_) => {
                    // Clean up temp file
                    let _ = tokio::fs::remove_file(uploaded_path).await;
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

    // Download standard manifests from GitHub
    let repo = config.repo.as_deref().unwrap_or("SteamAutoCracks/ManifestHub");
    let sha = config.sha.as_deref().unwrap_or(&config.app_id);

    for depot in &standard_depots {
        if check_cancelled(state, job_id).await {
            return Ok(());
        }

        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("downloading_manifest".to_string());
        event.depot_id = Some(depot.depot_id.clone());
        event.manifest_id = Some(depot.manifest_id.clone());
        emit_progress(app, &event);

        match manifest_downloader::download_manifest(
            &state.http_client,
            &config.app_id,
            &depot.depot_id,
            &depot.manifest_id,
            repo,
            sha,
            &work_dir,
            config.github_token.as_deref(),
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

    // Download custom manifests from ManifestHub API
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

    // Check if all manifests failed
    let success_count = manifest_results.iter().filter(|(_, s)| *s).count();
    if success_count == 0 && !manifest_results.is_empty() {
        let error_msg = "All manifest downloads failed".to_string();
        let mut event = ProgressEvent::new("error", job_id);
        event.message = Some(error_msg.clone());
        emit_progress(app, &event);
        return Ok(());
    }

    // Step 3: Generate depot keys
    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let mut event = ProgressEvent::new("status", job_id);
    event.step = Some("generating_keys".to_string());
    emit_progress(app, &event);

    // Collect depot keys from config
    let mut depot_infos: Vec<DepotInfo> = config
        .depots
        .iter()
        .map(|d| {
            let mut key = d.depot_key.clone();

            // Merge keyVdfKeys if available
            if key.is_none() {
                if let Some(ref kvk) = config.key_vdf_keys {
                    key = kvk.get(&d.depot_id).cloned();
                }
            }

            DepotInfo {
                depot_id: d.depot_id.parse().unwrap_or(0),
                depot_key: key,
                manifest_id: Some(d.custom_manifest_id.as_deref().unwrap_or(&d.manifest_id).to_string()),
            }
        })
        .collect();

    // If we have a repo with Key.vdf and some depots lack keys, try downloading
    if let Some(ref repo_name) = config.repo {
        if depot_infos.iter().any(|d| d.depot_key.is_none()) {
            if let Some(ref sha_val) = config.sha {
                let mut event = ProgressEvent::new("status", job_id);
                event.step = Some("downloading_keyvdf".to_string());
                emit_progress(app, &event);

                match manifest_downloader::download_key_vdf(
                    &state.http_client,
                    &config.app_id,
                    repo_name,
                    sha_val,
                    None,
                    config.github_token.as_deref(),
                )
                .await
                {
                    Ok(vdf_content) => {
                        let vdf_keys = vdf_parser::parse_key_vdf(&vdf_content, Some(repo_name));
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
        }
    }

    // Generate steam.keys file
    let keys_result = depot_keys_generator::generate_depot_keys(
        config.app_id.parse().unwrap_or(0),
        &depot_infos,
        Some(folder_name),
        base_dir,
    )
    .await?;

    let mut event = ProgressEvent::new("status", job_id);
    event.step = Some("keys_generated".to_string());
    event.depot_count = Some(keys_result.depot_count);
    emit_progress(app, &event);

    // Step 4: Run DepotDownloaderMod
    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let exe_path = depot_runner::get_exe_path_async().await?;

    // Filter to only depots with successful manifests
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

    // Load settings for extra args
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

    // Complete
    let dl_success_count = download_results.iter().filter(|r| r["success"].as_bool().unwrap_or(false)).count();
    let mut event = ProgressEvent::new("complete", job_id);
    event.message = Some(format!(
        "Download complete. {}/{} depots downloaded successfully.",
        dl_success_count,
        run_depots.len()
    ));
    event.results = Some(serde_json::Value::Array(download_results));
    emit_progress(app, &event);

    // Mark job as complete
    {
        let mut jobs = state.active_jobs.lock().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = "complete".to_string();
        }
    }

    Ok(())
}

/// Cancel an active download job.
#[command]
pub async fn cancel_download(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    job_id: String,
) -> Result<(), String> {
    // Check job exists and get the download dir
    let download_dir = {
        let jobs = state.active_jobs.lock().await;
        if !jobs.contains_key(&job_id) {
            return Err("Job not found".to_string());
        }
        jobs.get(&job_id).and_then(|j| j.download_dir.clone())
    };

    // Kill the process
    depot_runner::kill_job(&state, &job_id).await;

    // Emit cancellation event
    let mut event = ProgressEvent::new("cancelled", &job_id);
    event.message = Some("Download cancelled and files are being cleaned up.".to_string());
    emit_progress(&app, &event);

    // Clean up downloaded files
    if let Some(dir) = download_dir {
        let dir_path = std::path::PathBuf::from(&dir);
        if dir_path.exists() {
            // Wait a bit for the process to fully exit
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                // Try to delete the directory with retries
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

/// Export a script for manual download execution.
/// On Windows: generates a .bat file. On Linux: generates a .sh file.
#[command]
pub async fn export_batch_script(config: serde_json::Value) -> Result<String, String> {
    let app_id = config["appId"]
        .as_str()
        .or_else(|| config["mainAppId"].as_str())
        .ok_or("Missing appId")?;

    let depots = config["depots"]
        .as_array()
        .or_else(|| config["selectedDepots"].as_array())
        .ok_or("Missing depots array")?;

    let folder_name = config["folderName"]
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| app_id.to_string());

    let download_dir = config["downloadDir"]
        .as_str()
        .unwrap_or(".");

    let default_game_name = format!("App {}", app_id);
    let game_name = config["gameName"]
        .as_str()
        .unwrap_or(&default_game_name);

    #[cfg(target_os = "windows")]
    {
        // Escape special batch characters
        let safe_name = escape_batch_chars(game_name);

        let mut script = String::new();
        script.push_str("@echo off\r\n");
        script.push_str("echo === Steam Manifest Downloader - Batch Script ===\r\n");
        script.push_str(&format!("echo Game: {} (AppID: {})\r\n", safe_name, app_id));
        script.push_str(&format!("echo Download Directory: {}\r\n", download_dir));
        script.push_str("echo.\r\n");
        script.push_str("\r\n");
        script.push_str(&format!("cd /d \"{}\"\r\n", download_dir));
        script.push_str("\r\n");

        for (i, depot) in depots.iter().enumerate() {
            let depot_id = depot["depotId"]
                .as_str()
                .or_else(|| depot["depot_id"].as_str())
                .unwrap_or("0");

            let manifest_id = depot["customManifestId"]
                .as_str()
                .or_else(|| depot["manifestId"].as_str())
                .or_else(|| depot["manifest_id"].as_str())
                .unwrap_or("0");

            script.push_str(&format!("REM Depot {}\r\n", depot_id));
            script.push_str(&format!(
                "DepotDownloaderMod.exe -app {} -depot {} -manifest {} -depotkeys \"{}\\steam.keys\" -manifestfile \"{}\\{}_{}.manifest\"\r\n",
                app_id, depot_id, manifest_id, folder_name, folder_name, depot_id, manifest_id
            ));
            script.push_str(&format!(
                "if %errorlevel% neq 0 echo ERROR: Depot {} failed!\r\n",
                depot_id
            ));

            if i < depots.len() - 1 {
                script.push_str("\r\n");
            }
        }

        script.push_str("\r\n");
        script.push_str("echo.\r\n");
        script.push_str("echo === All downloads complete! ===\r\n");
        script.push_str("pause\r\n");

        Ok(script)
    }

    #[cfg(target_os = "linux")]
    {
        let mut script = String::new();
        script.push_str("#!/bin/bash\n");
        script.push_str("echo \"=== Steam Manifest Downloader - Shell Script ===\"\n");
        script.push_str(&format!("echo \"Game: {} (AppID: {})\"\n", game_name.replace('"', "\\\""), app_id));
        script.push_str(&format!("echo \"Download Directory: {}\"\n", download_dir));
        script.push_str("echo\n");
        script.push_str("\n");
        script.push_str(&format!("cd \"{}\" || exit 1\n", download_dir));
        script.push_str("\n");

        for (i, depot) in depots.iter().enumerate() {
            let depot_id = depot["depotId"]
                .as_str()
                .or_else(|| depot["depot_id"].as_str())
                .unwrap_or("0");

            let manifest_id = depot["customManifestId"]
                .as_str()
                .or_else(|| depot["manifestId"].as_str())
                .or_else(|| depot["manifest_id"].as_str())
                .unwrap_or("0");

            script.push_str(&format!("# Depot {}\n", depot_id));
            script.push_str(&format!(
                "./DepotDownloaderMod -app {} -depot {} -manifest {} -depotkeys \"{}/steam.keys\" -manifestfile \"{}/{}_{}.manifest\"\n",
                app_id, depot_id, manifest_id, folder_name, folder_name, depot_id, manifest_id
            ));
            script.push_str(&format!(
                "if [ $? -ne 0 ]; then echo \"ERROR: Depot {} failed!\"; fi\n",
                depot_id
            ));

            if i < depots.len() - 1 {
                script.push_str("\n");
            }
        }

        script.push_str("\n");
        script.push_str("echo\n");
        script.push_str("echo \"=== All downloads complete! ===\"\n");

        Ok(script)
    }
}

// --- Helper functions ---

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
fn escape_batch_chars(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' | '|' | '<' | '>' | '^' | '%' => {
                result.push('^');
                result.push(c);
            }
            _ => result.push(c),
        }
    }
    result
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
