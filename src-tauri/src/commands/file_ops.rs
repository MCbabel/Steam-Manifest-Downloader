use tauri::command;
use crate::services::lua_parser;
use crate::services::st_parser;

/// Parse a `.lua` or `.st` file at the given path.
/// Returns the parsed depot information as JSON, or a descriptive error when the
/// file is missing, has an unsupported extension, or does not contain any
/// recognisable entries.
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

    let parsed = match ext.as_str() {
        "lua" => {
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))?;
            lua_parser::parse_lua_file(&content)
                .map_err(|e| format!("Invalid .lua file: {}", e))?
        }
        "st" => {
            let buffer = tokio::fs::read(&path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))?;
            st_parser::parse_st_file(&buffer)
                .map_err(|e| format!("Invalid .st file: {}", e))?
        }
        _ => {
            return Err(format!(
                "Unsupported file type: .{}. Expected .lua or .st",
                ext
            ));
        }
    };

    serde_json::to_value(&parsed)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

/// Parse lua content passed as a string directly (used when the frontend has
/// already loaded the file).
#[command]
pub async fn parse_lua_content(
    content: String,
    filename: String,
) -> Result<serde_json::Value, String> {
    let ext = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // `.st` payloads are binary and cannot be parsed from a plain string, so we
    // accept only lua-style content here.
    if ext == "st" {
        return Err(
            ".st files must be parsed from disk (use parse_lua_file with a path)."
                .to_string(),
        );
    }

    let parsed = lua_parser::parse_lua_file(&content)
        .map_err(|e| format!("Invalid lua content: {}", e))?;

    serde_json::to_value(&parsed)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}