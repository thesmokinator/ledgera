use crate::settings::{read_settings, AppSettings};
use serde::Serialize;
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HledgerStatus {
    available: bool,
    version: String,
    message: String,
    resolved_path: String,
    source: String,
}

/// Checks whether the configured hledger executable can be invoked.
#[tauri::command]
pub(crate) fn check_hledger(app: AppHandle) -> Result<HledgerStatus, String> {
    let settings = read_settings(&app)?;
    let (executable, source) = hledger_executable_with_source(&settings);
    let output = Command::new(&executable).arg("--version").output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(HledgerStatus {
                available: true,
                version: version.clone(),
                message: version,
                resolved_path: executable,
                source,
            })
        }
        Ok(output) => Ok(HledgerStatus {
            available: false,
            version: String::new(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            resolved_path: executable,
            source,
        }),
        Err(error) => Ok(HledgerStatus {
            available: false,
            version: String::new(),
            message: error.to_string(),
            resolved_path: executable,
            source,
        }),
    }
}

pub(crate) fn hledger_executable(settings: &AppSettings) -> String {
    hledger_executable_with_source(settings).0
}

fn hledger_executable_with_source(settings: &AppSettings) -> (String, String) {
    let configured = settings.hledger_path.trim();
    if !configured.is_empty() {
        return (configured.to_string(), "configured".to_string());
    }

    if let Some(detected) = find_hledger_executable() {
        return (detected, "detected".to_string());
    }

    ("hledger".to_string(), "fallback".to_string())
}

/// Finds hledger in common installation folders and in the user's login shell PATH.
fn find_hledger_executable() -> Option<String> {
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin/hledger"),
        PathBuf::from("/usr/local/bin/hledger"),
        PathBuf::from("/usr/bin/hledger"),
    ];

    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join(".local/bin/hledger"));
        candidates.push(home.join(".cabal/bin/hledger"));
    }

    candidates
        .into_iter()
        .chain(
            login_shell_path_dirs()
                .into_iter()
                .map(|path| path.join("hledger")),
        )
        .find(|path| is_executable_file(path))
        .map(|path| path.to_string_lossy().to_string())
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
        && Command::new(path)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
}

fn login_shell_path_dirs() -> Vec<PathBuf> {
    let shell = env::var("shell").unwrap_or_else(|_| "/bin/zsh".to_string());
    [["-li", "-c", "echo $PATH"], ["-l", "-c", "echo $PATH"]]
        .into_iter()
        .find_map(|args| {
            Command::new(&shell)
                .args(args)
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
        })
        .and_then(|output| {
            output
                .lines()
                .rev()
                .map(str::trim)
                .find(|line| line.contains('/') && !line.is_empty())
                .map(|line| line.split(':').map(PathBuf::from).collect())
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::hledger_executable;
    use crate::settings::AppSettings;

    #[test]
    fn configured_hledger_path_overrides_detection() {
        let settings = AppSettings {
            journal_path: String::new(),
            hledger_path: "/custom/bin/hledger".to_string(),
            theme: "system".to_string(),
            language: "system".to_string(),
            power_user: false,
            default_commodity: String::new(),
            fetch_prices: false,
            commodity_symbols: Vec::new(),
            exclude_balances: String::new(),
            include_investments: String::new(),
            prefill_postings: false,
            modules: Default::default(),
        };

        assert_eq!(hledger_executable(&settings), "/custom/bin/hledger");
    }
}
