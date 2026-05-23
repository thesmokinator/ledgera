use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

const UPDATE_CHECK_INTERVAL_HOURS: i64 = 24;
const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/thesmokinator/ledgera/releases?per_page=20";

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
    let current_version = current_app_version(&app);

    let status = match fetch_latest_github_release(&current_version).await {
        Ok(Some(release)) => {
            let latest_version = normalize_version_tag(&release.tag_name);
            UpdateStatus {
                current_version: current_version.clone(),
                available: true,
                latest_version: Some(latest_version),
                release_url: Some(release.html_url),
                release_name: release.name,
                published_at: release.published_at,
                checked_at: Some(checked_at),
                source: "network".to_string(),
                error: None,
            }
        }
        Ok(None) => UpdateStatus {
            current_version,
            latest_version: None,
            available: false,
            release_url: None,
            release_name: None,
            published_at: None,
            checked_at: Some(checked_at),
            source: "network".to_string(),
            error: None,
        },
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

fn current_app_version(app: &AppHandle) -> String {
    app.package_info().version.to_string()
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

async fn fetch_latest_github_release(
    current_version: &str,
) -> Result<Option<GitHubRelease>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(GITHUB_RELEASES_URL)
        .header(reqwest::header::USER_AGENT, "Ledgera")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!("GitHub returned {}", response.status()));
    }

    let releases = response
        .json::<Vec<GitHubRelease>>()
        .await
        .map_err(|error| error.to_string())?;

    Ok(releases
        .into_iter()
        .filter(|release| !release.draft)
        .filter(|release| {
            version_is_newer(&normalize_version_tag(&release.tag_name), current_version)
        })
        .max_by(|left, right| {
            compare_versions(
                &normalize_version_tag(&left.tag_name),
                &normalize_version_tag(&right.tag_name),
            )
        }))
}

fn normalize_version_tag(value: &str) -> String {
    value.trim().trim_start_matches('v').to_string()
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    compare_versions(latest, current).is_gt()
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let left_version = ParsedVersion::parse(left);
    let right_version = ParsedVersion::parse(right);

    left_version.compare(&right_version)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVersion {
    core: Vec<u64>,
    prerelease: Vec<String>,
}

impl ParsedVersion {
    fn parse(value: &str) -> Self {
        let normalized = normalize_version_tag(value);
        let (core, prerelease) = normalized
            .split_once('-')
            .map_or((normalized.as_str(), ""), |(core, prerelease)| {
                (core, prerelease)
            });

        Self {
            core: core
                .split('.')
                .filter_map(|part| part.parse::<u64>().ok())
                .collect(),
            prerelease: prerelease
                .split('.')
                .filter(|part| !part.is_empty())
                .map(ToString::to_string)
                .collect(),
        }
    }

    fn compare(&self, other: &Self) -> std::cmp::Ordering {
        for index in 0..self.core.len().max(other.core.len()) {
            let left = self.core.get(index).copied().unwrap_or(0);
            let right = other.core.get(index).copied().unwrap_or(0);
            match left.cmp(&right) {
                std::cmp::Ordering::Equal => {}
                ordering => return ordering,
            }
        }

        match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => compare_prerelease(&self.prerelease, &other.prerelease),
        }
    }
}

fn compare_prerelease(left: &[String], right: &[String]) -> std::cmp::Ordering {
    for index in 0..left.len().max(right.len()) {
        let Some(left_part) = left.get(index) else {
            return std::cmp::Ordering::Less;
        };
        let Some(right_part) = right.get(index) else {
            return std::cmp::Ordering::Greater;
        };

        let left_number = left_part.parse::<u64>();
        let right_number = right_part.parse::<u64>();
        let ordering = match (left_number, right_number) {
            (Ok(left), Ok(right)) => left.cmp(&right),
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => left_part.cmp(right_part),
        };

        if !ordering.is_eq() {
            return ordering;
        }
    }

    std::cmp::Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::{compare_versions, normalize_version_tag, version_is_newer};

    #[test]
    fn normalizes_release_tags() {
        assert_eq!(normalize_version_tag("v1.2.3"), "1.2.3");
        assert_eq!(normalize_version_tag("1.2.3"), "1.2.3");
    }

    #[test]
    fn detects_newer_versions() {
        assert!(version_is_newer("1.2.0", "1.1.9"));
        assert!(version_is_newer("1.2.1", "1.2.0"));
        assert!(!version_is_newer("1.2.0", "1.2.0"));
        assert!(!version_is_newer("1.1.9", "1.2.0"));
    }

    #[test]
    fn compares_prerelease_versions() {
        assert!(version_is_newer("0.1.0-rc.2", "0.1.0-rc.1"));
        assert!(!version_is_newer("0.1.0-rc.1", "0.1.0-rc.2"));
        assert!(version_is_newer("0.1.0", "0.1.0-rc.2"));
        assert!(!version_is_newer("0.1.0-rc.2", "0.1.0"));
    }

    #[test]
    fn compares_numeric_and_text_prerelease_identifiers() {
        assert!(compare_versions("1.0.0-rc.10", "1.0.0-rc.2").is_gt());
        assert!(compare_versions("1.0.0-beta", "1.0.0-beta.1").is_lt());
        assert!(compare_versions("1.0.0-1", "1.0.0-alpha").is_lt());
    }
}
