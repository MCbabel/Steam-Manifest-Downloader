pub mod lua_parser;
pub mod st_parser;
pub mod vdf_parser;
pub mod multi_repo_search;
pub mod manifest_hub_api;
pub mod depot_sources;
pub mod depot_keys_generator;
pub mod depot_runner;
pub mod steam_store_api;
pub mod settings;
pub mod embedded_tools;
pub mod history;
pub mod telemetry;
pub mod emulator;
pub mod depot_info;
pub mod steam_library;
pub mod steamless;
pub mod steam_api_bypass;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::AppHandle;

pub struct AppState {
    #[allow(dead_code)]
    pub app_handle: AppHandle,
    pub active_jobs: Arc<Mutex<HashMap<String, JobInfo>>>,
    pub http_client: reqwest::Client,
    pub steam_cache: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    pub telemetry: Option<telemetry::Telemetry>,
}

pub struct JobInfo {
    pub status: String,
    pub child_pid: Option<u32>,
    pub download_dir: Option<String>,
    #[cfg(target_os = "windows")]
    pub job_object: Option<Arc<depot_runner::win_job::JobObject>>,
}

impl AppState {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            steam_cache: Arc::new(Mutex::new(HashMap::new())),
            telemetry: None,
        }
    }

    pub fn has_active_downloads(&self) -> bool {
        // try_lock so this stays callable from the UI thread; treat contention
        // as "active" because the mutex is only held during job mutation.
        if let Ok(jobs) = self.active_jobs.try_lock() {
            jobs.values().any(|j| j.status == "downloading" || j.status == "running")
        } else {
            true
        }
    }
}
