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
}

fn log_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("ledgera.log.jsonl"))
}

fn should_log(level: &str) -> bool {
    matches!(level, "error" | "warn" | "info")
}

pub(crate) fn log_event(app: &AppHandle, level: &str, code: &str, message: impl Into<String>) {
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
        message: message.into(),
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

pub(crate) fn log_error(app: &AppHandle, code: &str, context: &str, error: impl AsRef<str>) {
    log_event(app, "error", code, log_message_with_error(context, error));
}

pub(crate) fn log_warn(app: &AppHandle, code: &str, context: &str, warning: impl AsRef<str>) {
    log_event(app, "warn", code, log_message_with_error(context, warning));
}

pub(crate) fn log_message_with_error(context: &str, error: impl AsRef<str>) -> String {
    let error = format_error_for_log(error.as_ref());
    if error.is_empty() {
        context.to_string()
    } else {
        format!("{}\n\nError:\n{}", context, error)
    }
}

fn format_error_for_log(error: &str) -> String {
    let error = error.trim();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(error) else {
        return error.to_string();
    };

    let Some(object) = value.as_object() else {
        return error.to_string();
    };

    let mut parts = Vec::new();
    if let Some(code) = object.get("code").and_then(serde_json::Value::as_str) {
        parts.push(format!("Code: {}", code));
    }
    if let Some(message) = object.get("message").and_then(serde_json::Value::as_str) {
        parts.push(message.to_string());
    }
    if let Some(details) = object.get("details").and_then(serde_json::Value::as_str) {
        if !details.trim().is_empty() {
            parts.push(format!("Details:\n{}", details.trim()));
        }
    }
    if let Some(field_errors) = object
        .get("fieldErrors")
        .and_then(serde_json::Value::as_array)
    {
        if !field_errors.is_empty() {
            let rendered = field_errors
                .iter()
                .map(|field_error| field_error.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            parts.push(format!("Field errors:\n{}", rendered));
        }
    }

    if parts.is_empty() {
        error.to_string()
    } else {
        parts.join("\n")
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
        .filter(|entry| should_log(&entry.level))
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
    use super::{filter_log_entries, log_message_with_error, should_log};

    #[test]
    fn should_log_returns_true_for_supported_levels() {
        assert!(should_log("error"));
        assert!(should_log("warn"));
        assert!(should_log("info"));
        assert!(!should_log("notice"));
        assert!(!should_log("debug"));
        assert!(!should_log(""));
    }

    #[test]
    fn filter_log_entries_returns_supported_levels() {
        let content = r#"
{"ts":"2025-01-01T00:00:00.000Z","level":"info","code":"TEST","message":"info msg"}
{"ts":"2025-01-01T00:00:01.000Z","level":"warn","code":"TEST","message":"warn msg"}
{"ts":"2025-01-01T00:00:02.000Z","level":"error","code":"TEST","message":"error msg"}
{"ts":"2025-01-01T00:00:03.000Z","level":"debug","code":"DEBUG","message":"debug msg"}
"#;

        let entries = filter_log_entries(content);

        assert_eq!(
            entries.len(),
            3,
            "should return info, warn, and error entries"
        );
        assert_eq!(entries[0].level, "info");
        assert_eq!(entries[1].level, "warn");
        assert_eq!(entries[2].level, "error");
    }

    #[test]
    fn filter_log_entries_ignores_unsupported_levels() {
        let content = r#"
{"ts":"2025-01-01T00:00:00.000Z","level":"debug","code":"TEST","message":"debug msg"}
{"ts":"2025-01-01T00:00:01.000Z","level":"notice","code":"TEST","message":"notice msg"}
"#;

        let entries = filter_log_entries(content);
        assert!(
            entries.is_empty(),
            "should be empty when no supported levels exist"
        );
    }

    #[test]
    fn filter_log_entries_ignores_malformed_lines() {
        let content =
            "not json\n{\"level\":\"error\",\"code\":\"OK\",\"message\":\"good\",\"ts\":\"0\"}\n";
        let entries = filter_log_entries(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "good");
    }

    #[test]
    fn log_message_with_error_appends_error_block() {
        assert_eq!(
            log_message_with_error("Failed to do thing.", "stack line"),
            "Failed to do thing.\n\nError:\nstack line",
        );
    }

    #[test]
    fn log_message_with_error_expands_structured_app_errors() {
        assert_eq!(
            log_message_with_error(
                "Failed to do thing.",
                r#"{"code":"boom","message":"Top level","details":"stack line"}"#,
            ),
            "Failed to do thing.\n\nError:\nCode: boom\nTop level\nDetails:\nstack line",
        );
    }
}
