use bzip2::read::BzDecoder;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tar::Archive;
use walkdir::WalkDir;

const RELEASES_API: &str = "https://api.github.com/repos/Detanup01/gbe_fork/releases/latest";
const USER_AGENT: &str = "SteamManifestDownloader";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    Regular,
    Experimental,
}

impl Variant {
    fn folder(self) -> &'static str {
        match self {
            Variant::Regular => "regular",
            Variant::Experimental => "experimental",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub tag: String,
    pub published_at: String,
    pub asset_url: String,
    pub asset_name: String,
    pub asset_size: u64,
    pub cached: bool,
    pub cache_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedFile {
    pub path: String,
    pub arch: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceResult {
    pub path: String,
    pub backup_path: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

fn cache_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("gbe_fork_cache")
}

fn asset_filename_for_platform() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "emu-linux-release.tar.bz2"
    }
    #[cfg(target_os = "windows")]
    {
        "emu-win-release.7z"
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        "emu-linux-release.tar.bz2"
    }
}

pub async fn fetch_release_info(client: &Client, app_data_dir: &Path) -> Result<ReleaseInfo, String> {
    let resp = client
        .get(RELEASES_API)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("GitHub fetch failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub returned HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("GitHub JSON parse failed: {}", e))?;

    let tag = body
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or("Missing tag_name in release info")?
        .to_string();
    let published_at = body
        .get("published_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let target_name = asset_filename_for_platform();
    let assets = body
        .get("assets")
        .and_then(|v| v.as_array())
        .ok_or("Missing assets array")?;
    let asset = assets
        .iter()
        .find(|a| a.get("name").and_then(|v| v.as_str()) == Some(target_name))
        .ok_or_else(|| format!("Asset {} not present in release", target_name))?;

    let asset_url = asset
        .get("browser_download_url")
        .and_then(|v| v.as_str())
        .ok_or("Asset has no browser_download_url")?
        .to_string();
    let asset_size = asset.get("size").and_then(|v| v.as_u64()).unwrap_or(0);

    let cache_dir = cache_root(app_data_dir).join(&tag);
    let cached = cache_dir.join(".extracted").exists();

    Ok(ReleaseInfo {
        tag,
        published_at,
        asset_url,
        asset_name: target_name.to_string(),
        asset_size,
        cached,
        cache_dir: cache_dir.to_string_lossy().to_string(),
    })
}

pub async fn ensure_cached(
    client: &Client,
    _app_data_dir: &Path,
    info: &ReleaseInfo,
) -> Result<PathBuf, String> {
    let cache_dir = PathBuf::from(&info.cache_dir);
    let extracted_marker = cache_dir.join(".extracted");
    if extracted_marker.exists() {
        return Ok(cache_dir);
    }

    fs::create_dir_all(&cache_dir).map_err(|e| format!("create cache dir: {}", e))?;
    let archive_path = cache_dir.join(&info.asset_name);

    if !archive_path.exists() {
        let mut resp = client
            .get(&info.asset_url)
            .header("User-Agent", USER_AGENT)
            .timeout(Duration::from_secs(300))
            .send()
            .await
            .map_err(|e| format!("download failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("download returned HTTP {}", resp.status()));
        }
        let tmp_path = cache_dir.join(format!("{}.part", info.asset_name));
        {
            let mut file =
                BufWriter::new(File::create(&tmp_path).map_err(|e| format!("create archive: {}", e))?);
            while let Some(chunk) = resp.chunk().await.map_err(|e| format!("chunk: {}", e))? {
                file.write_all(&chunk).map_err(|e| format!("write archive: {}", e))?;
            }
            file.flush().map_err(|e| format!("flush archive: {}", e))?;
        }
        fs::rename(&tmp_path, &archive_path).map_err(|e| format!("finalize archive: {}", e))?;
    }

    extract_archive(&archive_path, &cache_dir)?;
    fs::write(&extracted_marker, info.tag.as_bytes())
        .map_err(|e| format!("write extracted marker: {}", e))?;

    Ok(cache_dir)
}

fn extract_archive(archive: &Path, dst: &Path) -> Result<(), String> {
    let name = archive
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if name.ends_with(".tar.bz2") {
        extract_tar_bz2(archive, dst)
    } else if name.ends_with(".7z") {
        extract_7z(archive, dst)
    } else {
        Err(format!("unsupported archive extension: {}", name))
    }
}

fn extract_tar_bz2(archive: &Path, dst: &Path) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("open archive: {}", e))?;
    let mut decoder = BzDecoder::new(BufReader::new(file));
    let mut tmp = Vec::with_capacity(64 * 1024 * 1024);
    decoder
        .read_to_end(&mut tmp)
        .map_err(|e| format!("bzip2 decompress: {}", e))?;
    let mut tar = Archive::new(io::Cursor::new(tmp));
    tar.unpack(dst).map_err(|e| format!("tar unpack: {}", e))?;
    Ok(())
}

fn extract_7z(archive: &Path, dst: &Path) -> Result<(), String> {
    sevenz_rust::decompress_file(archive, dst).map_err(|e| format!("7z decompress: {}", e))
}

fn release_root(cache_dir: &Path) -> PathBuf {
    cache_dir.join("release")
}

pub fn dll_source(cache_dir: &Path, variant: Variant, x64: bool) -> Result<PathBuf, String> {
    let root = release_root(cache_dir).join(variant.folder());
    #[cfg(target_os = "windows")]
    {
        let file = if x64 { "steam_api64.dll" } else { "steam_api.dll" };
        let p = root.join(if x64 { "x64" } else { "x32" }).join(file);
        if p.exists() {
            Ok(p)
        } else {
            Err(format!("emulator dll not found at {}", p.display()))
        }
    }
    #[cfg(target_os = "linux")]
    {
        let _ = x64;
        let p = root.join(if x64 { "x64" } else { "x32" }).join("libsteam_api.so");
        if p.exists() {
            Ok(p)
        } else {
            Err(format!("emulator so not found at {}", p.display()))
        }
    }
}

pub fn generate_interfaces_tool(cache_dir: &Path, x64: bool) -> Result<PathBuf, String> {
    let root = release_root(cache_dir).join("tools").join("generate_interfaces");
    #[cfg(target_os = "windows")]
    let name = if x64 {
        "generate_interfaces_x64.exe"
    } else {
        "generate_interfaces_x32.exe"
    };
    #[cfg(not(target_os = "windows"))]
    let name = if x64 {
        "generate_interfaces_x64"
    } else {
        "generate_interfaces_x32"
    };
    let p = root.join(name);
    if p.exists() {
        Ok(p)
    } else {
        Err(format!("generate_interfaces tool not found at {}", p.display()))
    }
}

pub fn lobby_connect_tool(cache_dir: &Path, x64: bool) -> Result<PathBuf, String> {
    let root = release_root(cache_dir).join("tools").join("lobby_connect");
    #[cfg(target_os = "windows")]
    let name = if x64 {
        "lobby_connect_x64.exe"
    } else {
        "lobby_connect_x32.exe"
    };
    #[cfg(not(target_os = "windows"))]
    let name = if x64 {
        "lobby_connect_x64"
    } else {
        "lobby_connect_x32"
    };
    let p = root.join(name);
    if p.exists() {
        Ok(p)
    } else {
        Err(format!("lobby_connect tool not found at {}", p.display()))
    }
}

pub fn scan_game_dir(game_dir: &Path) -> Vec<ScannedFile> {
    let mut out = Vec::new();
    if !game_dir.exists() {
        return out;
    }
    for entry in WalkDir::new(game_dir).follow_links(false).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let (arch, kind) = if name == "steam_api64.dll" {
            ("x64", "steam_api")
        } else if name == "steam_api.dll" {
            ("x32", "steam_api")
        } else if name == "libsteam_api.so" {
            (detect_so_arch(entry.path()), "steam_api")
        } else {
            continue;
        };
        out.push(ScannedFile {
            path: entry.path().to_string_lossy().to_string(),
            arch: arch.to_string(),
            kind: kind.to_string(),
        });
    }
    out
}

fn detect_so_arch(path: &Path) -> &'static str {
    let Ok(mut file) = File::open(path) else {
        return "x64";
    };
    let mut header = [0u8; 5];
    if file.read_exact(&mut header).is_err() {
        return "x64";
    }
    if &header[..4] != b"\x7fELF" {
        return "x64";
    }
    match header[4] {
        1 => "x32",
        2 => "x64",
        _ => "x64",
    }
}

pub fn run_generate_interfaces(tool: &Path, dll: &Path) -> Result<String, String> {
    let work_dir = dll.parent().ok_or("dll has no parent dir")?;
    let output = std::process::Command::new(tool)
        .current_dir(work_dir)
        .arg(dll.file_name().ok_or("dll has no file name")?)
        .output()
        .map_err(|e| format!("spawn generate_interfaces: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "generate_interfaces exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let interfaces_file = work_dir.join("steam_interfaces.txt");
    if !interfaces_file.exists() {
        return Err(format!(
            "generate_interfaces did not produce steam_interfaces.txt in {}",
            work_dir.display()
        ));
    }
    let contents =
        fs::read_to_string(&interfaces_file).map_err(|e| format!("read interfaces file: {}", e))?;
    Ok(contents)
}

pub fn apply_replacement(
    target: &Path,
    cache_dir: &Path,
    variant: Variant,
    app_id: &str,
    installed_app_ids: &[String],
) -> ReplaceResult {
    let mut result = ReplaceResult {
        path: target.to_string_lossy().to_string(),
        backup_path: None,
        success: false,
        error: None,
    };

    let arch = arch_of_file(target);
    let x64 = arch == "x64";

    let emu_dll = match dll_source(cache_dir, variant, x64) {
        Ok(p) => p,
        Err(e) => {
            result.error = Some(e);
            return result;
        }
    };

    let tool = match generate_interfaces_tool(cache_dir, x64) {
        Ok(p) => p,
        Err(e) => {
            result.error = Some(e);
            return result;
        }
    };

    let interfaces = match run_generate_interfaces(&tool, target) {
        Ok(contents) => contents,
        Err(e) => {
            result.error = Some(format!("generate_interfaces failed: {}", e));
            return result;
        }
    };

    let backup = backup_path_for(target);
    if !backup.exists() {
        if let Err(e) = fs::copy(target, &backup) {
            result.error = Some(format!("backup original: {}", e));
            return result;
        }
    }
    result.backup_path = Some(backup.to_string_lossy().to_string());

    if let Err(e) = fs::copy(&emu_dll, target) {
        result.error = Some(format!("copy emulator dll: {}", e));
        return result;
    }

    let parent = match target.parent() {
        Some(p) => p,
        None => {
            result.error = Some("target has no parent dir".into());
            return result;
        }
    };
    let settings_dir = parent.join("steam_settings");
    if let Err(e) = fs::create_dir_all(&settings_dir) {
        result.error = Some(format!("create steam_settings: {}", e));
        return result;
    }

    if let Err(e) = fs::write(settings_dir.join("steam_interfaces.txt"), interfaces) {
        result.error = Some(format!("write steam_interfaces.txt: {}", e));
        return result;
    }
    if let Err(e) = fs::write(settings_dir.join("steam_appid.txt"), app_id.trim()) {
        result.error = Some(format!("write steam_appid.txt: {}", e));
        return result;
    }
    if !installed_app_ids.is_empty() {
        let joined = installed_app_ids
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if let Err(e) = fs::write(settings_dir.join("installed_app_ids.txt"), joined) {
            result.error = Some(format!("write installed_app_ids.txt: {}", e));
            return result;
        }
    }

    result.success = true;
    result
}

pub fn revert_replacement(target: &Path) -> Result<(), String> {
    let backup = backup_path_for(target);
    if !backup.exists() {
        return Err(format!("no backup found at {}", backup.display()));
    }
    fs::copy(&backup, target).map_err(|e| format!("restore from backup: {}", e))?;
    let _ = fs::remove_file(&backup);
    if let Some(parent) = target.parent() {
        let settings_dir = parent.join("steam_settings");
        let _ = fs::remove_file(settings_dir.join("steam_interfaces.txt"));
        let _ = fs::remove_file(settings_dir.join("steam_appid.txt"));
        let _ = fs::remove_file(settings_dir.join("installed_app_ids.txt"));
        let _ = fs::remove_dir(&settings_dir);
    }
    Ok(())
}

fn backup_path_for(target: &Path) -> PathBuf {
    let mut p = target.as_os_str().to_owned();
    p.push(".steam.bak");
    PathBuf::from(p)
}

fn arch_of_file(target: &Path) -> &'static str {
    let name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name == "steam_api64.dll" {
        return "x64";
    }
    if name == "steam_api.dll" {
        return "x32";
    }
    if name == "libsteam_api.so" {
        return detect_so_arch(target);
    }
    "x64"
}

