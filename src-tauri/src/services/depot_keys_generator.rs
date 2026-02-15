use std::path::Path;
use tokio::fs;

use crate::services::lua_parser::DepotInfo;

/// Result of generating depot keys file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DepotKeysResult {
    pub output_path: String,
    pub depot_count: usize,
}

/// Generate `steam.keys` file content in format: `depotId;hexKey\n`
/// and write it to the specified directory.
///
/// # Arguments
/// * `app_id` - The Steam app ID
/// * `depots` - List of depot info with keys
/// * `folder_name` - Optional folder name (defaults to app_id string)
/// * `base_dir` - Base directory for output
pub async fn generate_depot_keys(
    app_id: u64,
    depots: &[DepotInfo],
    folder_name: Option<&str>,
    base_dir: &Path,
) -> Result<DepotKeysResult, String> {
    let folder = folder_name
        .map(|s| s.to_string())
        .unwrap_or_else(|| app_id.to_string());
    let output_dir = base_dir.join(&folder);
    let output_path = output_dir.join("steam.keys");

    // Ensure output directory exists
    fs::create_dir_all(&output_dir)
        .await
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Build file content: depotId;hexKey per line
    let lines: Vec<String> = depots
        .iter()
        .filter_map(|depot| {
            depot
                .depot_key
                .as_ref()
                .map(|key| format!("{};{}", depot.depot_id, key))
        })
        .collect();

    let content = if lines.is_empty() {
        String::from("\n")
    } else {
        lines.join("\n") + "\n"
    };

    fs::write(&output_path, &content)
        .await
        .map_err(|e| format!("Failed to write steam.keys: {}", e))?;

    Ok(DepotKeysResult {
        output_path: output_path.to_string_lossy().to_string(),
        depot_count: lines.len(),
    })
}

/// Generate depot keys content as a string without writing to file.
#[allow(dead_code)]
pub fn generate_depot_keys_content(depots: &[DepotInfo]) -> String {
    let lines: Vec<String> = depots
        .iter()
        .filter_map(|depot| {
            depot
                .depot_key
                .as_ref()
                .map(|key| format!("{};{}", depot.depot_id, key))
        })
        .collect();

    if lines.is_empty() {
        String::from("\n")
    } else {
        lines.join("\n") + "\n"
    }
}
