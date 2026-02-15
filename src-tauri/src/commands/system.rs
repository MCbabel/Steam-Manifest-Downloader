use tauri::command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Check if .NET 9 runtime is installed.
/// On Linux, the DDM binary is self-contained so dotnet is not needed.
/// Runs `dotnet --list-runtimes` and checks for "Microsoft.NETCore.App 9."
#[command]
pub async fn check_dotnet() -> Result<serde_json::Value, String> {
    // On Linux, DDM is a self-contained binary — no dotnet needed
    #[cfg(target_os = "linux")]
    {
        return Ok(serde_json::json!({
            "installed": true,
            "version": "self-contained",
        }));
    }

    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("dotnet");
        cmd.args(["--list-runtimes"]);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Look for .NET 9.x runtime
                let mut found_version: Option<String> = None;

                for line in stdout.lines() {
                    if line.contains("Microsoft.NETCore.App 9.") {
                        // Extract version: "Microsoft.NETCore.App 9.0.1 [path]"
                        if let Some(version_part) = line.strip_prefix("Microsoft.NETCore.App ") {
                            if let Some(ver) = version_part.split_whitespace().next() {
                                found_version = Some(ver.to_string());
                            }
                        }
                        break;
                    }
                }

                Ok(serde_json::json!({
                    "installed": found_version.is_some(),
                    "version": found_version,
                }))
            }
            Err(_) => {
                // dotnet command not found
                Ok(serde_json::json!({
                    "installed": false,
                    "version": null,
                }))
            }
        }
    }
}

/// Get disk space information for a given path.
/// Uses PowerShell on Windows, statvfs on Linux.
#[command]
pub async fn get_disk_space(path: String) -> Result<serde_json::Value, String> {
    #[cfg(target_os = "windows")]
    {
        if path.len() < 2 {
            return Err("Invalid path".to_string());
        }

        let drive_letter = path.chars().next().ok_or("Empty path")?;

        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
                "-NoProfile",
                "-Command",
                &format!(
                    "$d = Get-PSDrive {}; @{{ Free = $d.Free; Used = $d.Used }} | ConvertTo-Json",
                    drive_letter
                ),
            ]);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let data: serde_json::Value = serde_json::from_str(stdout.trim())
                    .unwrap_or_else(|_| serde_json::json!({}));

                let free = data["Free"].as_u64().unwrap_or(0);
                let used = data["Used"].as_u64().unwrap_or(0);
                let total = free + used;
                let free_gb = (free as f64) / (1024.0 * 1024.0 * 1024.0);
                let free_gb = (free_gb * 100.0).round() / 100.0;

                Ok(serde_json::json!({
                    "free": free,
                    "total": total,
                    "freeGB": free_gb,
                    "drive": format!("{}:", drive_letter),
                    "path": path,
                }))
            }
            Err(e) => Err(format!("Failed to check disk space: {}", e)),
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::ffi::CString;

        let c_path = CString::new(path.clone())
            .map_err(|_| "Invalid path".to_string())?;

        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            let result = libc::statvfs(c_path.as_ptr(), &mut stat);
            if result != 0 {
                return Err("Failed to get filesystem stats".to_string());
            }

            let free = (stat.f_bavail as u64) * (stat.f_frsize as u64);
            let total = (stat.f_blocks as u64) * (stat.f_frsize as u64);
            let free_gb = (free as f64) / (1024.0 * 1024.0 * 1024.0);
            let free_gb = (free_gb * 100.0).round() / 100.0;

            Ok(serde_json::json!({
                "free": free,
                "total": total,
                "freeGB": free_gb,
                "drive": path.clone(),
                "path": path,
            }))
        }
    }
}
