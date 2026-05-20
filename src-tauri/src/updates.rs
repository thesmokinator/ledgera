use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

const UPDATE_CHECK_INTERVAL_HOURS: i64 = 24;
const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/thesmokinator/ledgera/releases/latest";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateStatus {
    current_version: String,
    latest_version: Option<String>,
    available: bool,
    release_url: Option<String>,
    release_name: Option<String>,
    published_at: Option<String>,
    checked_at: Option<String>,
    source: String,
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    html_url: String,
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// Checks GitHub Releases for a newer Ledgera version, caching automatic checks daily.
#[tauri::command]
pub(crate) async fn check_for_updates(
    app: AppHandle,
    force: Option<bool>,
) -> Result<UpdateStatus, String> {
    let force = force.unwrap_or(false);
    let cache_path = update_status_path(&app)?;

    if !force {
        if let Some(cached) = read_cached_update_status(&cache_path) {
            if update_cache_is_fresh(&cached) {
                return Ok(UpdateStatus {
                    source: "cache".to_string(),
                    ..cached
                });
            }
        }
    }

    let checked_at = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let current_version = current_app_version();

    let status = match fetch_latest_github_release().await {
        Ok(release) => {
            let latest_version = normalize_version_tag(&release.tag_name);
            UpdateStatus {
                current_version: current_version.clone(),
                available: version_is_newer(&latest_version, &current_version),
                latest_version: Some(latest_version),
                release_url: Some(release.html_url),
                release_name: release.name,
                published_at: release.published_at,
                checked_at: Some(checked_at),
                source: "network".to_string(),
                error: None,
            }
        }
        Err(error) => UpdateStatus {
            current_version,
            latest_version: None,
            available: false,
            release_url: None,
            release_name: None,
            published_at: None,
            checked_at: Some(checked_at),
            source: "network".to_string(),
            error: Some(error),
        },
    };

    let _ = write_cached_update_status(&cache_path, &status);
    Ok(status)
}

fn update_status_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("update-status.json"))
}

fn current_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn read_cached_update_status(path: &Path) -> Option<UpdateStatus> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_cached_update_status(path: &Path, status: &UpdateStatus) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let content = serde_json::to_string_pretty(status).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

fn update_cache_is_fresh(status: &UpdateStatus) -> bool {
    let Some(checked_at) = &status.checked_at else {
        return false;
    };
    let Ok(checked_at) = chrono::DateTime::parse_from_rfc3339(checked_at) else {
        return false;
    };
    Utc::now().signed_duration_since(checked_at.with_timezone(&Utc))
        < chrono::Duration::hours(UPDATE_CHECK_INTERVAL_HOURS)
}

async fn fetch_latest_github_release() -> Result<GitHubRelease, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(GITHUB_LATEST_RELEASE_URL)
        .header(reqwest::header::USER_AGENT, "Ledgera")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!("GitHub returned {}", response.status()));
    }

    let release = response
        .json::<GitHubRelease>()
        .await
        .map_err(|error| error.to_string())?;

    if release.draft || release.prerelease {
        return Err("Latest release is not a stable public release.".to_string());
    }

    Ok(release)
}

fn normalize_version_tag(value: &str) -> String {
    value.trim().trim_start_matches('v').to_string()
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    let latest_parts = parse_version_parts(latest);
    let current_parts = parse_version_parts(current);

    for index in 0..latest_parts.len().max(current_parts.len()) {
        let latest_part = latest_parts.get(index).copied().unwrap_or(0);
        let current_part = current_parts.get(index).copied().unwrap_or(0);
        if latest_part > current_part {
            return true;
        }
        if latest_part < current_part {
            return false;
        }
    }

    false
}

fn parse_version_parts(value: &str) -> Vec<u64> {
    value
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{normalize_version_tag, parse_version_parts, version_is_newer};

    #[test]
    fn normalizes_release_tags() {
        assert_eq!(normalize_version_tag("v1.2.3"), "1.2.3");
        assert_eq!(normalize_version_tag("1.2.3"), "1.2.3");
    }

    #[test]
    fn parses_numeric_version_parts() {
        assert_eq!(parse_version_parts("1.2.3"), vec![1, 2, 3]);
        assert_eq!(parse_version_parts("v1.2.3-beta.4"), vec![1, 2, 3, 4]);
    }

    #[test]
    fn detects_newer_versions() {
        assert!(version_is_newer("1.2.0", "1.1.9"));
        assert!(version_is_newer("1.2.1", "1.2.0"));
        assert!(!version_is_newer("1.2.0", "1.2.0"));
        assert!(!version_is_newer("1.1.9", "1.2.0"));
    }
}
