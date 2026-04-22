use tauri::command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
// SMD_BUILD_CHANNEL / SMD_GIT_SHA / SMD_BUILD_DATE are injected by CI
// (dev-build.yml → "dev", release.yml → "stable"). Unset → "dev-local".
#[command]
pub fn get_build_info() -> serde_json::Value {
    let channel = option_env!("SMD_BUILD_CHANNEL").unwrap_or("dev-local");
    let git_sha = option_env!("SMD_GIT_SHA").unwrap_or("unknown");
    let build_date = option_env!("SMD_BUILD_DATE").unwrap_or("unknown");

    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };

    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "channel": channel,
        "gitSha": git_sha,
        "buildDate": build_date,
        "profile": profile,
        "targetOs": std::env::consts::OS,
        "targetArch": std::env::consts::ARCH,
    })
}

#[command]
pub async fn check_dotnet() -> Result<serde_json::Value, String> {
    // DDM is shipped self-contained on Linux.
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
                let mut found_version: Option<String> = None;

                for line in stdout.lines() {
                    if line.contains("Microsoft.NETCore.App 9.") {
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
            Err(_) => Ok(serde_json::json!({
                "installed": false,
                "version": null,
            })),
        }
    }
}

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
