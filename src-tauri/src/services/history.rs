use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

const MAX_HISTORY_ENTRIES: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub app_id: String,
    pub game_name: Option<String>,
    pub header_image: Option<String>,
    pub depot_count: usize,
    pub depots_downloaded: usize,
    /// "complete", "partial", "failed"
    pub status: String,
    pub download_dir: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub source_repo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
}

fn history_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("history.json")
}

pub async fn load_history(app_data_dir: &Path) -> History {
    let path = history_path(app_data_dir);
    match fs::read_to_string(&path).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => History::default(),
    }
}

async fn save_history(app_data_dir: &Path, history: &History) -> Result<(), String> {
    let path = history_path(app_data_dir);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create history directory: {}", e))?;
    }

    let content = serde_json::to_string_pretty(history)
        .map_err(|e| format!("Failed to serialize history: {}", e))?;

    fs::write(&path, content)
        .await
        .map_err(|e| format!("Failed to write history file: {}", e))?;

    Ok(())
}

pub async fn add_entry(app_data_dir: &Path, entry: HistoryEntry) -> Result<(), String> {
    let mut history = load_history(app_data_dir).await;

    // Insert at the beginning (newest first)
    history.entries.insert(0, entry);

    // Trim to max entries
    if history.entries.len() > MAX_HISTORY_ENTRIES {
        history.entries.truncate(MAX_HISTORY_ENTRIES);
    }

    save_history(app_data_dir, &history).await
}

pub async fn remove_entry(app_data_dir: &Path, entry_id: &str) -> Result<(), String> {
    let mut history = load_history(app_data_dir).await;
    history.entries.retain(|e| e.id != entry_id);
    save_history(app_data_dir, &history).await
}

pub async fn clear(app_data_dir: &Path) -> Result<(), String> {
    let history = History::default();
    save_history(app_data_dir, &history).await
}
