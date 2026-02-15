use reqwest::Client;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Build authorization headers for GitHub raw downloads.
fn build_auth_header(token: Option<&str>) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", "SteamManifestDownloader".parse().unwrap());
    if let Some(t) = token {
        if !t.is_empty() {
            headers.insert(
                "Authorization",
                format!("Bearer {}", t).parse().unwrap(),
            );
        }
    }
    headers
}

/// Download a `.manifest` file from a GitHub repo.
///
/// URL pattern: `https://raw.githubusercontent.com/{repo}/{sha_or_appid}/{depot_id}_{manifest_id}.manifest`
/// Saves to: `{output_dir}/{depot_id}_{manifest_id}.manifest`
pub async fn download_manifest(
    client: &Client,
    app_id: &str,
    depot_id: &str,
    manifest_id: &str,
    repo: &str,
    sha: &str,
    output_dir: &Path,
    token: Option<&str>,
) -> Result<PathBuf, String> {
    let filename = format!("{}_{}.manifest", depot_id, manifest_id);
    // Use app_id as branch reference for raw URLs
    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}",
        repo, app_id, filename
    );

    // Ensure output directory exists
    fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let output_path = output_dir.join(&filename);

    let response = client
        .get(&url)
        .headers(build_auth_header(token))
        .send()
        .await
        .map_err(|e| format!("Failed to download manifest {}: {}", filename, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download manifest for depot {}: {} {}",
            depot_id,
            response.status(),
            response.status().canonical_reason().unwrap_or("")
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read manifest response body: {}", e))?;

    fs::write(&output_path, &bytes)
        .await
        .map_err(|e| format!("Failed to write manifest file: {}", e))?;

    // sha is available for reference but raw URLs use branch name (app_id)
    let _ = sha;

    Ok(output_path)
}

/// Download Key.vdf from a repo branch.
///
/// Returns the VDF file content as a string.
pub async fn download_key_vdf(
    client: &Client,
    app_id: &str,
    repo: &str,
    _sha: &str,
    filename: Option<&str>,
    token: Option<&str>,
) -> Result<String, String> {
    let vdf_filename = filename.unwrap_or("Key.vdf");
    download_repo_text_file(client, repo, app_id, vdf_filename, token).await
}

/// Download any text file from a repo branch using raw GitHub URL.
///
/// URL: `https://raw.githubusercontent.com/{repo}/{branch}/{filename}`
pub async fn download_repo_text_file(
    client: &Client,
    repo: &str,
    branch: &str,
    filename: &str,
    token: Option<&str>,
) -> Result<String, String> {
    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}",
        repo, branch, filename
    );

    let response = client
        .get(&url)
        .headers(build_auth_header(token))
        .send()
        .await
        .map_err(|e| format!("Failed to download {}: {}", filename, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download {}: {} {}",
            filename,
            response.status(),
            response.status().canonical_reason().unwrap_or("")
        ));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read text response for {}: {}", filename, e))
}
