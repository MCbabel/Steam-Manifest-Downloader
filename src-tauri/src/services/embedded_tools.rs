use std::path::PathBuf;
use tokio::fs;

// ---------------------------------------------------------------------------
// Platform-specific embedded DepotDownloaderMod files
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod platform {
    pub const DDM_FILES: &[(&str, &[u8])] = &[
        ("DepotDownloaderMod.exe", include_bytes!("../../../DepotDownloaderMod-Windows/DepotDownloaderMod.exe")),
        ("DepotDownloaderMod.dll", include_bytes!("../../../DepotDownloaderMod-Windows/DepotDownloaderMod.dll")),
        ("DepotDownloaderMod.deps.json", include_bytes!("../../../DepotDownloaderMod-Windows/DepotDownloaderMod.deps.json")),
        ("DepotDownloaderMod.runtimeconfig.json", include_bytes!("../../../DepotDownloaderMod-Windows/DepotDownloaderMod.runtimeconfig.json")),
        ("SteamKit2.dll", include_bytes!("../../../DepotDownloaderMod-Windows/SteamKit2.dll")),
        ("protobuf-net.Core.dll", include_bytes!("../../../DepotDownloaderMod-Windows/protobuf-net.Core.dll")),
        ("protobuf-net.dll", include_bytes!("../../../DepotDownloaderMod-Windows/protobuf-net.dll")),
        ("QRCoder.dll", include_bytes!("../../../DepotDownloaderMod-Windows/QRCoder.dll")),
        ("System.IO.Hashing.dll", include_bytes!("../../../DepotDownloaderMod-Windows/System.IO.Hashing.dll")),
        ("ZstdSharp.dll", include_bytes!("../../../DepotDownloaderMod-Windows/ZstdSharp.dll")),
    ];
    pub const EXE_NAME: &str = "DepotDownloaderMod.exe";
}

#[cfg(target_os = "linux")]
mod platform {
    pub const DDM_FILES: &[(&str, &[u8])] = &[
        ("DepotDownloaderMod", include_bytes!("../../../DepotDownloaderMod-linux-full/DepotDownloaderMod")),
    ];
    pub const EXE_NAME: &str = "DepotDownloaderMod";
}

/// Extract embedded DepotDownloaderMod files to a directory.
/// Returns the path to the DepotDownloaderMod executable.
/// Uses a marker file to avoid re-extracting on every run.
pub async fn ensure_extracted() -> Result<PathBuf, String> {
    // Use the system temp directory + app-specific subfolder
    let base_dir = std::env::temp_dir().join("SteamManifestDownloader").join("DepotDownloaderMod");

    let marker_file = base_dir.join(".extracted");

    // Check if already extracted (marker file exists and exe exists)
    let exe_path = base_dir.join(platform::EXE_NAME);
    if marker_file.exists() && exe_path.exists() {
        return Ok(exe_path);
    }

    // Extract all files
    eprintln!("[EmbeddedTools] Extracting DepotDownloaderMod to {:?}", base_dir);

    fs::create_dir_all(&base_dir)
        .await
        .map_err(|e| format!("Failed to create extraction directory: {}", e))?;

    for (name, data) in platform::DDM_FILES {
        let file_path = base_dir.join(name);
        fs::write(&file_path, data)
            .await
            .map_err(|e| format!("Failed to extract {}: {}", name, e))?;
    }

    // On Linux, set executable permissions
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(&exe_path).await
            .map_err(|e| format!("Failed to get metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        tokio::fs::set_permissions(&exe_path, perms).await
            .map_err(|e| format!("Failed to set executable permission: {}", e))?;
    }

    // Write marker file
    fs::write(&marker_file, "extracted")
        .await
        .map_err(|e| format!("Failed to write marker file: {}", e))?;

    eprintln!("[EmbeddedTools] Extraction complete");
    Ok(exe_path)
}
