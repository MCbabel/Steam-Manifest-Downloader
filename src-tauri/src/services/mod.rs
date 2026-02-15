pub mod lua_parser;
pub mod st_parser;
pub mod vdf_parser;
pub mod github_api;
pub mod multi_repo_search;
pub mod alternative_sources;
pub mod manifest_downloader;
pub mod manifest_hub_api;
pub mod depot_keys_generator;
pub mod depot_runner;
pub mod steam_store_api;
pub mod settings;
pub mod embedded_tools;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::AppHandle;

pub struct AppState {
    #[allow(dead_code)] // Stored for potential future use; currently only set during construction
    pub app_handle: AppHandle,
    pub active_jobs: Arc<Mutex<HashMap<String, JobInfo>>>,
    pub http_client: reqwest::Client,
    pub steam_cache: Arc<Mutex<HashMap<String, serde_json::Value>>>,
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
        }
    }

    pub fn has_active_downloads(&self) -> bool {
        // Use try_lock to avoid blocking the UI thread
        if let Ok(jobs) = self.active_jobs.try_lock() {
            jobs.values().any(|j| j.status == "downloading" || j.status == "running")
        } else {
            true // Assume active if we can't check
        }
    }
}
