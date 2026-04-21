use tauri::command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[command]
pub async fn is_shortcut_supported() -> Result<serde_json::Value, String> {
    #[cfg(target_os = "windows")]
    { Ok(serde_json::json!({ "supported": true })) }
    #[cfg(not(target_os = "windows"))]
    { Ok(serde_json::json!({ "supported": false })) }
}

/// Scan a download directory for executable files and rank them by likelihood
/// of being the main game executable.
#[command]
pub async fn detect_executables(download_dir: String) -> Result<serde_json::Value, String> {
    let dir_path = std::path::PathBuf::from(&download_dir);
    if !dir_path.exists() {
        return Err("Download directory does not exist".to_string());
    }

    let mut exes: Vec<(String, String, u64)> = Vec::new(); // (path, name, size)
    scan_dir_recursive(&dir_path, &mut exes, 0, 5).await;

    if exes.is_empty() {
        return Ok(serde_json::json!({ "executables": [] }));
    }

    // Extract game name hint from folder name (e.g. "220 - Half-Life 2" -> "half-life 2")
    let folder_name = dir_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let game_hint = folder_name
        .split(" - ")
        .nth(1)
        .unwrap_or(&folder_name)
        .to_lowercase();
    let hint_words: Vec<&str> = game_hint
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .collect();

    // Score each executable
    let mut scored: Vec<(i64, String, String, u64)> = exes
        .into_iter()
        .map(|(path, name, size)| {
            let name_lower = name.to_lowercase();
            let mut score: i64 = 0;

            // Blacklist known non-game executables
            if is_blacklisted(&name_lower) {
                score -= 10000;
            }

            // Name match bonus
            for word in &hint_words {
                if name_lower.contains(word) {
                    score += 500;
                }
            }

            // Size bonus (MB scale)
            score += (size / (1024 * 1024)) as i64;

            (score, path, name, size)
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.0.cmp(&a.0));

    let executables: Vec<serde_json::Value> = scored
        .iter()
        .enumerate()
        .map(|(i, (_, path, name, size))| {
            serde_json::json!({
                "path": path,
                "name": name,
                "size": size,
                "recommended": i == 0,
            })
        })
        .collect();

    Ok(serde_json::json!({ "executables": executables }))
}

/// Recursively scan for .exe files.
async fn scan_dir_recursive(
    dir: &std::path::Path,
    results: &mut Vec<(String, String, u64)>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth || results.len() >= 1000 {
        return;
    }

    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(_) => return,
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if let Ok(metadata) = entry.metadata().await {
            if metadata.is_dir() {
                Box::pin(scan_dir_recursive(&path, results, depth + 1, max_depth)).await;
            } else if metadata.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.to_lowercase().ends_with(".exe") {
                    results.push((
                        path.to_string_lossy().to_string(),
                        name,
                        metadata.len(),
                    ));
                }
            }
        }

        if results.len() >= 1000 {
            break;
        }
    }
}

/// Check if an executable name matches known non-game executables.
fn is_blacklisted(name_lower: &str) -> bool {
    const BLACKLIST_EXACT: &[&str] = &[
        "unins000.exe",
        "unins001.exe",
        "uninstall.exe",
        "setup.exe",
        "installer.exe",
        "dxsetup.exe",
        "dxwebsetup.exe",
    ];

    const BLACKLIST_PREFIX: &[&str] = &[
        "unitycrashhandler",
        "ue4prereqsetup",
        "vcredist",
        "dotnetfx",
        "directx",
        "crashreporter",
        "crashhandler",
        "bugreporter",
    ];

    const BLACKLIST_CONTAINS: &[&str] = &[
        "redist",
        "prerequisite",
        "dotnet",
    ];

    if BLACKLIST_EXACT.contains(&name_lower) {
        return true;
    }

    for prefix in BLACKLIST_PREFIX {
        if name_lower.starts_with(prefix) {
            return true;
        }
    }

    for needle in BLACKLIST_CONTAINS {
        if name_lower.contains(needle) {
            return true;
        }
    }

    false
}

/// Create desktop and/or start menu shortcuts for a game executable.
/// Windows-only: uses PowerShell WScript.Shell COM to create .lnk files.
#[command]
pub async fn create_shortcuts(
    exe_path: String,
    game_name: String,
    icon_path: Option<String>,
    create_desktop: bool,
    create_start_menu: bool,
) -> Result<serde_json::Value, String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (&exe_path, &game_name, &icon_path, create_desktop, create_start_menu);
        return Err("Shortcut creation is only supported on Windows".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let exe = std::path::PathBuf::from(&exe_path);
        if !exe.exists() {
            return Err("Executable not found".to_string());
        }

        let working_dir = exe
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let icon = icon_path.unwrap_or_else(|| exe_path.clone());

        // Sanitize game name for filename
        let safe_name: String = game_name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' })
            .collect();

        let mut desktop_ok = false;
        let mut start_menu_ok = false;
        let mut errors: Vec<String> = Vec::new();

        if create_desktop {
            match create_lnk_shortcut(&safe_name, &exe_path, &working_dir, &icon, &game_name, "Desktop") {
                Ok(_) => desktop_ok = true,
                Err(e) => errors.push(format!("Desktop: {}", e)),
            }
        }

        if create_start_menu {
            match create_lnk_shortcut(&safe_name, &exe_path, &working_dir, &icon, &game_name, "Programs") {
                Ok(_) => start_menu_ok = true,
                Err(e) => errors.push(format!("Start Menu: {}", e)),
            }
        }

        Ok(serde_json::json!({
            "desktop": desktop_ok,
            "startMenu": start_menu_ok,
            "errors": errors,
        }))
    }
}

/// Create a .lnk shortcut using PowerShell on Windows.
///
/// All user-derived values are passed through environment variables rather than
/// being interpolated into the PowerShell script. This keeps the script body
/// constant and prevents any value from being parsed as PowerShell code.
#[cfg(target_os = "windows")]
fn create_lnk_shortcut(
    safe_name: &str,
    exe_path: &str,
    working_dir: &str,
    icon_path: &str,
    description: &str,
    folder_type: &str, // "Desktop" or "Programs"
) -> Result<(), String> {
    const SCRIPT: &str = "\
        $ErrorActionPreference = 'Stop';\
        $folder = [Environment]::GetFolderPath($env:SMD_FOLDER);\
        $target = Join-Path $folder ($env:SMD_NAME + '.lnk');\
        $ws = New-Object -ComObject WScript.Shell;\
        $s = $ws.CreateShortcut($target);\
        $s.TargetPath = $env:SMD_TARGET;\
        $s.WorkingDirectory = $env:SMD_WORKDIR;\
        $s.Description = $env:SMD_DESC;\
        $s.IconLocation = $env:SMD_ICON;\
        $s.Save()";

    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT]);
    cmd.env("SMD_FOLDER", folder_type);
    cmd.env("SMD_NAME", safe_name);
    cmd.env("SMD_TARGET", exe_path);
    cmd.env("SMD_WORKDIR", working_dir);
    cmd.env("SMD_DESC", description);
    cmd.env("SMD_ICON", icon_path);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("PowerShell error: {}", stderr.trim()))
            }
        }
        Err(e) => Err(format!("Failed to run PowerShell: {}", e)),
    }
}
