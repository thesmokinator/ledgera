use crate::app_error::to_error_string_with_details;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

const LOG_RETENTION_DAYS: i64 = 90;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LogEntry {
    ts: String,
    level: String,
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

fn log_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("ledgera.log.jsonl"))
}

fn should_log(level: &str) -> bool {
    level == "error"
}

pub(crate) fn log_event_with_details(
    app: &AppHandle,
    level: &str,
    code: &str,
    message: &str,
    details: Option<String>,
) {
    if !should_log(level) {
        return;
    }
    let path = match log_path(app) {
        Ok(p) => p,
        Err(_) => return,
    };
    let entry = LogEntry {
        ts: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        level: level.to_string(),
        code: code.to_string(),
        message: message.to_string(),
        details,
    };
    if let Ok(json) = serde_json::to_string(&entry) {
        let mut line = json;
        line.push('\n');
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()));
    }
}

pub(crate) fn cleanup_old_logs(app: &AppHandle) {
    let path = match log_path(app) {
        Ok(p) => p,
        Err(_) => return,
    };
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let cutoff = Utc::now() - chrono::Duration::days(LOG_RETENTION_DAYS);
    let cutoff_str = cutoff.format("%Y-%m-%dT").to_string();
    let kept: Vec<&str> = content
        .lines()
        .filter(|line| line.trim() >= cutoff_str.as_str())
        .collect();
    if kept.len() != content.lines().count() {
        let _ = fs::write(&path, kept.join("\n") + "\n");
    }
}

fn filter_log_entries(content: &str) -> Vec<LogEntry> {
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<LogEntry>(line).ok())
        .filter(|entry| entry.level == "error")
        .collect()
}

#[tauri::command]
pub(crate) fn get_logs(app: AppHandle) -> Result<Vec<LogEntry>, String> {
    let path = log_path(&app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|error| {
        to_error_string_with_details(
            "log_read_failed",
            "Unable to read log file.",
            error.to_string(),
        )
    })?;
    let mut entries = filter_log_entries(&content);
    entries.reverse();
    Ok(entries)
}

#[tauri::command]
pub(crate) fn clear_logs(app: AppHandle) -> Result<(), String> {
    let path = log_path(&app)?;
    fs::write(&path, "").map_err(|error| {
        to_error_string_with_details(
            "log_write_failed",
            "Unable to clear log file.",
            error.to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{filter_log_entries, should_log};

    #[test]
    fn should_log_returns_true_only_for_error() {
        assert!(should_log("error"));
        assert!(!should_log("info"));
        assert!(!should_log("warn"));
        assert!(!should_log("notice"));
        assert!(!should_log("debug"));
        assert!(!should_log(""));
    }

    #[test]
    fn filter_log_entries_returns_only_error_entries() {
        let content = r#"
{"ts":"2025-01-01T00:00:00.000Z","level":"info","code":"TEST","message":"info msg"}
{"ts":"2025-01-01T00:00:01.000Z","level":"warn","code":"TEST","message":"warn msg"}
{"ts":"2025-01-01T00:00:02.000Z","level":"error","code":"TEST","message":"error msg"}
{"ts":"2025-01-01T00:00:03.000Z","level":"error","code":"ERR2","message":"another error"}
"#;

        let entries = filter_log_entries(content);

        assert_eq!(entries.len(), 2, "should return only error entries");
        assert_eq!(entries[0].level, "error");
        assert_eq!(entries[0].message, "error msg");
        assert_eq!(entries[1].level, "error");
        assert_eq!(entries[1].message, "another error");
    }

    #[test]
    fn filter_log_entries_returns_empty_when_no_errors() {
        let content = r#"
{"ts":"2025-01-01T00:00:00.000Z","level":"info","code":"TEST","message":"info msg"}
{"ts":"2025-01-01T00:00:01.000Z","level":"warn","code":"TEST","message":"warn msg"}
"#;

        let entries = filter_log_entries(content);
        assert!(entries.is_empty(), "should be empty when no errors");
    }

    #[test]
    fn filter_log_entries_ignores_malformed_lines() {
        let content =
            "not json\n{\"level\":\"error\",\"code\":\"OK\",\"message\":\"good\",\"ts\":\"0\"}\n";
        let entries = filter_log_entries(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "good");
    }
}
