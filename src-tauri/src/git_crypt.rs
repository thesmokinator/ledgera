use crate::{journal, settings::read_settings};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::AppHandle;

const GITATTRIBUTES_RULE: &str = "*.journal filter=git-crypt diff=git-crypt";

/// Known installation paths for git-crypt across platforms.
/// Used as fallback when the binary is not on PATH (e.g., Tauri bundled app on macOS).
const COMMON_GIT_CRYPT_PATHS: &[&str] = &[
    "/opt/homebrew/bin/git-crypt",                    // macOS Apple Silicon Homebrew
    "/usr/local/bin/git-crypt",                       // macOS Intel Homebrew / Linux
    "/home/linuxbrew/.linuxbrew/bin/git-crypt",       // Linuxbrew
    "/usr/bin/git-crypt",                             // Linux (apt, pacman, etc.)
];

/// Returns the full path to the git-crypt binary, if found.
///
/// First tries PATH lookup via `command -v` (Unix) or `where` (Windows).
/// Falls back to common installation directories to handle
/// bundled desktop apps with a limited PATH (e.g. Tauri on macOS).
pub fn find_git_crypt_path() -> Option<PathBuf> {
    // Try PATH lookup first
    #[cfg(unix)]
    {
        if let Ok(output) = Command::new("sh")
            .args(["-c", "command -v git-crypt"])
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }

    #[cfg(windows)]
    {
        if let Ok(output) = Command::new("where").arg("git-crypt.exe").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }

    // Fallback: known installation paths for bundled apps
    for path in COMMON_GIT_CRYPT_PATHS {
        let p = Path::new(path);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }

    None
}

fn is_installed() -> bool {
    Command::new("git-crypt")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_active(journal_dir: &Path) -> bool {
    let attr_path = journal_dir.join(".gitattributes");
    if !attr_path.exists() {
        return false;
    }
    fs::read_to_string(&attr_path)
        .map(|content| content.contains(GITATTRIBUTES_RULE))
        .unwrap_or(false)
}

fn journal_repo_path(settings: &crate::settings::AppSettings) -> Option<PathBuf> {
    let journal_path = journal::files::require_journal_path(settings).ok()?;
    let journal_dir = journal_path.parent()?;

    let output = Command::new("git")
        .current_dir(journal_dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let repo_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if repo_root.is_empty() {
        return None;
    }
    Some(PathBuf::from(repo_root))
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitCryptStatus {
    pub(crate) installed: bool,
    pub(crate) enabled: bool,
}

#[tauri::command]
pub(crate) fn git_crypt_status(app: AppHandle) -> Result<GitCryptStatus, String> {
    let settings = read_settings(&app)?;
    let installed = is_installed();

    if settings.journal_path.trim().is_empty() {
        return Ok(GitCryptStatus {
            installed,
            enabled: false,
        });
    }

    let enabled = journal_repo_path(&settings)
        .as_ref()
        .map(|p| is_active(p))
        .unwrap_or(false);

    Ok(GitCryptStatus { installed, enabled })
}


