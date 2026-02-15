use tauri::command;
use crate::services::lua_parser;
use crate::services::st_parser;

/// Parse a .lua or .st file at the given path.
/// Returns the parsed depot information as JSON.
#[command]
pub async fn parse_lua_file(path: String) -> Result<serde_json::Value, String> {
    let file_path = std::path::Path::new(&path);

    if !file_path.exists() {
        return Err(format!("File not found: {}", path));
    }

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "lua" => {
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))?;
            let result = lua_parser::parse_lua_file(&content);
            serde_json::to_value(&result).map_err(|e| format!("Failed to serialize result: {}", e))
        }
        "st" => {
            let buffer = tokio::fs::read(&path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))?;
            let result = st_parser::parse_st_file(&buffer)?;
            serde_json::to_value(&result).map_err(|e| format!("Failed to serialize result: {}", e))
        }
        _ => Err(format!("Unsupported file type: .{}. Expected .lua or .st", ext)),
    }
}

/// Parse lua content string directly (for when frontend passes content).
#[command]
pub async fn parse_lua_content(content: String, filename: String) -> Result<serde_json::Value, String> {
    let ext = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "lua" | "" => {
            let result = lua_parser::parse_lua_file(&content);
            serde_json::to_value(&result).map_err(|e| format!("Failed to serialize result: {}", e))
        }
        "st" => {
            // For .st files, content should be base64 encoded or raw bytes
            // Try parsing as UTF-8 lua first, then as raw bytes
            let result = lua_parser::parse_lua_file(&content);
            serde_json::to_value(&result).map_err(|e| format!("Failed to serialize result: {}", e))
        }
        _ => {
            // Default: try lua parsing
            let result = lua_parser::parse_lua_file(&content);
            serde_json::to_value(&result).map_err(|e| format!("Failed to serialize result: {}", e))
        }
    }
}
