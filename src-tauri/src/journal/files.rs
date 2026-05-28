use crate::{
    app_error::{to_error_string, to_error_string_with_details},
    journal::util::split_inline_comment,
    settings::AppSettings,
};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub(crate) struct JournalFile {
    pub(crate) path: PathBuf,
    pub(crate) content: String,
}

/// Resolves the configured journal path.
pub(crate) fn require_journal_path(settings: &AppSettings) -> Result<PathBuf, String> {
    if settings.journal_path.trim().is_empty() {
        return Err(to_error_string(
            "journal_not_configured",
            "Configure a journal path in Settings before loading transactions.",
        ));
    }

    let path = PathBuf::from(settings.journal_path.trim());
    if !path.exists() {
        return Err(to_error_string_with_details(
            "journal_not_found",
            "Journal file does not exist.",
            format!("Expected at: {}", path.display()),
        ));
    }
    Ok(path)
}

pub(crate) fn load_journal_files(journal_path: &Path) -> Result<Vec<JournalFile>, String> {
    let mut files = Vec::new();
    let mut visited = HashSet::new();
    load_journal_file_recursive(journal_path, &mut visited, &mut files)?;
    Ok(files)
}

fn load_journal_file_recursive(
    journal_path: &Path,
    visited: &mut HashSet<PathBuf>,
    files: &mut Vec<JournalFile>,
) -> Result<(), String> {
    let canonical_path =
        fs::canonicalize(journal_path).unwrap_or_else(|_| journal_path.to_path_buf());
    if !visited.insert(canonical_path.clone()) {
        return Ok(());
    }

    let content = fs::read_to_string(&canonical_path).map_err(|error| {
        to_error_string_with_details(
            "journal_read_failed",
            "Unable to read journal file.",
            format!("{}: {}", canonical_path.display(), error),
        )
    })?;
    let include_paths = find_include_paths(&canonical_path, &content)?;
    files.push(JournalFile {
        path: canonical_path,
        content,
    });

    for include_path in include_paths {
        load_journal_file_recursive(&include_path, visited, files)?;
    }

    Ok(())
}

fn find_include_paths(journal_path: &Path, content: &str) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for line in content.lines() {
        if let Some(include) = parse_include_directive(line) {
            paths.extend(resolve_include_paths(journal_path, &include)?);
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

pub(crate) fn parse_include_directive(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
        return None;
    }

    let include = trimmed.strip_prefix("include")?.trim_start();
    if include.is_empty() {
        return None;
    }

    let include = split_inline_comment(include).0.trim();
    Some(include.trim_matches('"').to_string())
}

fn resolve_include_paths(journal_path: &Path, include: &str) -> Result<Vec<PathBuf>, String> {
    let base_dir = journal_path.parent().unwrap_or_else(|| Path::new("."));
    let include_path = PathBuf::from(include);
    let absolute_pattern = if include_path.is_absolute() {
        include_path
    } else {
        base_dir.join(include_path)
    };

    if !include.contains('*') {
        return if absolute_pattern.exists() {
            Ok(vec![absolute_pattern])
        } else {
            Err(to_error_string_with_details(
                "journal_include_missing",
                "Included journal file does not exist.",
                format!("Expected at: {}", absolute_pattern.display()),
            ))
        };
    }

    expand_simple_glob(&absolute_pattern)
}

fn expand_simple_glob(pattern: &Path) -> Result<Vec<PathBuf>, String> {
    let parent = pattern.parent().unwrap_or_else(|| Path::new("."));
    let Some(file_pattern) = pattern.file_name().and_then(|name| name.to_str()) else {
        return Ok(Vec::new());
    };

    if !parent.exists() {
        return Ok(Vec::new());
    }

    let mut matches = fs::read_dir(parent)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| wildcard_matches(file_pattern, name))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    matches.sort();
    Ok(matches)
}

fn wildcard_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let parts = pattern.split('*').collect::<Vec<_>>();
    if parts.len() == 1 {
        return pattern == value;
    }

    let mut remainder = value;
    if let Some(first) = parts.first() {
        if !first.is_empty() {
            let Some(stripped) = remainder.strip_prefix(first) else {
                return false;
            };
            remainder = stripped;
        }
    }

    for part in parts.iter().skip(1).take(parts.len().saturating_sub(2)) {
        if part.is_empty() {
            continue;
        }
        let Some(index) = remainder.find(part) else {
            return false;
        };
        remainder = &remainder[index + part.len()..];
    }

    if let Some(last) = parts.last() {
        last.is_empty() || remainder.ends_with(last)
    } else {
        true
    }
}

