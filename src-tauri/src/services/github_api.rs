use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const USER_AGENT: &str = "SteamManifestDownloader";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchCheckResult {
    pub exists: bool,
    pub branch: Option<String>,
    pub last_updated: Option<String>,
    pub sha: Option<String>,
    pub error: Option<String>,
    pub rate_limited: bool,
}

#[allow(dead_code)] // Available for future rate-limit checking features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    pub remaining: u64,
    pub limit: u64,
    pub reset_time: String,
}

/// Build headers for GitHub API requests.
fn build_headers(token: Option<&str>) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", "application/vnd.github.v3+json".parse().unwrap());
    headers.insert("User-Agent", USER_AGENT.parse().unwrap());
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

/// Check if response indicates rate limiting.
fn is_rate_limited(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS
}

/// Check if a branch exists for the given app_id on the default ManifestHub repo.
pub async fn check_branch(
    client: &Client,
    app_id: &str,
    token: Option<&str>,
) -> Result<BranchCheckResult, String> {
    let url = format!(
        "https://api.github.com/repos/SteamAutoCracks/ManifestHub/branches/{}",
        app_id
    );

    let response = client
        .get(&url)
        .headers(build_headers(token))
        .send()
        .await
        .map_err(|e| format!("GitHub API request failed: {}", e))?;

    let status = response.status();

    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(BranchCheckResult {
            exists: false,
            branch: None,
            last_updated: None,
            sha: None,
            error: Some(format!("Branch not found for AppID {}", app_id)),
            rate_limited: false,
        });
    }

    if is_rate_limited(status) {
        let reset = response
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<i64>().ok())
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| "unknown".to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        return Ok(BranchCheckResult {
            exists: false,
            branch: None,
            last_updated: None,
            sha: None,
            error: Some(format!("GitHub API rate limit exceeded. Resets at {}", reset)),
            rate_limited: true,
        });
    }

    if !status.is_success() {
        return Ok(BranchCheckResult {
            exists: false,
            branch: None,
            last_updated: None,
            sha: None,
            error: Some(format!("GitHub API error: {}", status)),
            rate_limited: false,
        });
    }

    let data: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub response: {}", e))?;

    let branch_name = data["name"].as_str().map(String::from);
    let last_updated = data["commit"]["commit"]["committer"]["date"]
        .as_str()
        .map(String::from);
    let sha = data["commit"]["sha"].as_str().map(String::from);

    Ok(BranchCheckResult {
        exists: true,
        branch: branch_name,
        last_updated,
        sha,
        error: None,
        rate_limited: false,
    })
}

/// Get git tree for a repo at a given SHA.
pub async fn get_tree(
    client: &Client,
    repo: &str,
    sha: &str,
    token: Option<&str>,
) -> Result<Value, String> {
    let url = format!(
        "https://api.github.com/repos/{}/git/trees/{}",
        repo, sha
    );

    let response = client
        .get(&url)
        .headers(build_headers(token))
        .send()
        .await
        .map_err(|e| format!("GitHub Tree API request failed: {}", e))?;

    let status = response.status();

    if is_rate_limited(status) {
        return Err("GitHub API rate limit exceeded".to_string());
    }

    if !status.is_success() {
        return Err(format!("GitHub Tree API error: {}", status));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse tree response: {}", e))
}

/// Check GitHub API rate limit status.
#[allow(dead_code)]
pub async fn check_rate_limit(
    client: &Client,
    token: Option<&str>,
) -> Result<RateLimitInfo, String> {
    let url = "https://api.github.com/rate_limit";

    let response = client
        .get(url)
        .headers(build_headers(token))
        .send()
        .await
        .map_err(|e| format!("GitHub rate limit API request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub rate limit API error: {}",
            response.status()
        ));
    }

    let data: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse rate limit response: {}", e))?;

    let core = data
        .get("resources")
        .and_then(|r| r.get("core"))
        .or_else(|| data.get("rate"))
        .ok_or("Missing rate limit data in response")?;

    let remaining = core["remaining"].as_u64().unwrap_or(0);
    let limit = core["limit"].as_u64().unwrap_or(0);
    let reset_ts = core["reset"].as_i64().unwrap_or(0);
    let reset_time = chrono::DateTime::from_timestamp(reset_ts, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(RateLimitInfo {
        remaining,
        limit,
        reset_time,
    })
}

/// Get branch info for any repo (not just the default one).
pub async fn get_branch_info(
    client: &Client,
    repo: &str,
    app_id: &str,
    token: Option<&str>,
) -> Result<BranchCheckResult, String> {
    let url = format!(
        "https://api.github.com/repos/{}/branches/{}",
        repo, app_id
    );

    let response = client
        .get(&url)
        .headers(build_headers(token))
        .send()
        .await
        .map_err(|e| format!("GitHub API request failed: {}", e))?;

    let status = response.status();

    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(BranchCheckResult {
            exists: false,
            branch: None,
            last_updated: None,
            sha: None,
            error: Some(format!("Branch {} not found in {}", app_id, repo)),
            rate_limited: false,
        });
    }

    if is_rate_limited(status) {
        return Ok(BranchCheckResult {
            exists: false,
            branch: None,
            last_updated: None,
            sha: None,
            error: Some("GitHub API rate limit exceeded".to_string()),
            rate_limited: true,
        });
    }

    if !status.is_success() {
        return Ok(BranchCheckResult {
            exists: false,
            branch: None,
            last_updated: None,
            sha: None,
            error: Some(format!("GitHub API error: {}", status)),
            rate_limited: false,
        });
    }

    let data: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub response: {}", e))?;

    let branch_name = data["name"].as_str().map(String::from);
    let last_updated = data["commit"]["commit"]["author"]["date"]
        .as_str()
        .map(String::from);
    let sha = data["commit"]["sha"].as_str().map(String::from);

    Ok(BranchCheckResult {
        exists: true,
        branch: branch_name,
        last_updated,
        sha,
        error: None,
        rate_limited: false,
    })
}
