use crate::{
    app_error::{to_error_string, to_error_string_with_details},
    journal::files::{load_journal_files, require_journal_path},
    logs,
    settings::read_settings,
};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitSyncFileStatus {
    path: String,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitSyncStatus {
    available: bool,
    repo_found: bool,
    repo_root: Option<String>,
    branch: Option<String>,
    upstream: Option<String>,
    remote: Option<String>,
    ahead: u32,
    behind: u32,
    dirty: bool,
    files: Vec<GitSyncFileStatus>,
    last_commit: Option<String>,
    error: Option<String>,
}

#[tauri::command]
pub(crate) fn git_sync_status(app: AppHandle) -> Result<GitSyncStatus, String> {
    match git_sync_status_for_app(&app) {
        Ok(status) => Ok(status),
        Err(error) => {
            logs::log_event_with_details(&app, "error", "git_sync_status_failed", &error, None);
            Ok(error_status(error))
        }
    }
}

#[tauri::command]
pub(crate) fn git_pull_journal(app: AppHandle) -> Result<GitSyncStatus, String> {
    let result = git_pull_journal_for_app(&app);
    if let Err(error) = &result {
        logs::log_event_with_details(&app, "error", "git_pull_failed", error, None);
    }
    result
}

#[tauri::command]
pub(crate) fn git_commit_and_push_journal(
    app: AppHandle,
    message: String,
) -> Result<GitSyncStatus, String> {
    let result = git_commit_and_push_journal_for_app(&app, &message);
    if let Err(error) = &result {
        logs::log_event_with_details(&app, "error", "git_commit_push_failed", error, None);
    }
    result
}

fn git_sync_status_for_app(app: &AppHandle) -> Result<GitSyncStatus, String> {
    let context = git_context(app)?;
    Ok(build_git_status(&context))
}

fn error_status(error: String) -> GitSyncStatus {
    GitSyncStatus {
        available: false,
        repo_found: false,
        repo_root: None,
        branch: None,
        upstream: None,
        remote: None,
        ahead: 0,
        behind: 0,
        dirty: false,
        files: Vec::new(),
        last_commit: None,
        error: Some(error),
    }
}

fn git_pull_journal_for_app(app: &AppHandle) -> Result<GitSyncStatus, String> {
    let context = git_context(app)?;
    ensure_repo_ready(&context)?;

    let current = build_git_status(&context);
    if current.dirty {
        return Err(to_error_string(
            "git_sync_dirty",
            "Commit or discard journal changes before pulling.",
        ));
    }

    run_git_checked(&context.repo_root, &["pull", "--ff-only"])?;
    Ok(build_git_status(&context))
}

fn git_commit_and_push_journal_for_app(
    app: &AppHandle,
    message: &str,
) -> Result<GitSyncStatus, String> {
    let message = validate_commit_message(message)?;
    let context = git_context(app)?;
    ensure_repo_ready(&context)?;

    let current = build_git_status(&context);
    if current.behind > 0 {
        return Err(to_error_string(
            "git_sync_behind",
            "Pull remote changes before committing and pushing.",
        ));
    }
    if current.files.is_empty() {
        return Ok(current);
    }

    let mut add_args = vec!["add", "--"];
    let journal_files = context
        .journal_files
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    add_args.extend(journal_files.iter().copied());
    run_git_checked(&context.repo_root, &add_args)?;

    let mut diff_args = vec!["diff", "--cached", "--quiet", "--"];
    diff_args.extend(journal_files.iter().copied());
    let diff_output = run_git(&context.repo_root, &diff_args)?;
    if diff_output.status == 0 {
        return Ok(build_git_status(&context));
    }
    if diff_output.status != 1 {
        return Err(command_error("git diff --cached", &diff_output));
    }

    let mut commit_args = vec!["commit", "-m", message, "--"];
    commit_args.extend(journal_files.iter().copied());
    run_git_checked(&context.repo_root, &commit_args)?;
    run_git_checked(&context.repo_root, &["push"])?;
    Ok(build_git_status(&context))
}

struct GitContext {
    repo_root: PathBuf,
    journal_files: Vec<String>,
}

struct GitOutput {
    status: i32,
    stdout: String,
    stderr: String,
}

fn git_context(app: &AppHandle) -> Result<GitContext, String> {
    let settings = read_settings(app)?;
    let journal_path = require_journal_path(&settings)?;
    let journal_dir = journal_path.parent().unwrap_or_else(|| Path::new("."));

    let repo_output = run_git(journal_dir, &["rev-parse", "--show-toplevel"])?;
    if repo_output.status != 0 {
        return Err(to_error_string_with_details(
            "git_repo_not_found",
            "The configured journal is not inside a Git repository.",
            repo_output.stderr,
        ));
    }

    let repo_root = PathBuf::from(repo_output.stdout.trim());
    let files = load_journal_files(&journal_path)?;
    let mut journal_files = files
        .iter()
        .filter_map(|file| relative_to_repo(&repo_root, &file.path))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    if journal_files.is_empty() {
        if let Some(path) = relative_to_repo(&repo_root, &journal_path) {
            journal_files.push(path);
        }
    }

    Ok(GitContext {
        repo_root,
        journal_files,
    })
}

fn relative_to_repo(repo_root: &Path, path: &Path) -> Option<String> {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let repo_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    path.strip_prefix(repo_root)
        .ok()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
}

fn ensure_repo_ready(context: &GitContext) -> Result<(), String> {
    if context.journal_files.is_empty() {
        return Err(to_error_string(
            "git_no_journal_files",
            "No journal files are available for Git sync.",
        ));
    }
    Ok(())
}

fn build_git_status(context: &GitContext) -> GitSyncStatus {
    let branch = run_git_text(&context.repo_root, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let upstream = run_git_text(
        &context.repo_root,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    );
    let remote = upstream
        .as_deref()
        .and_then(|value| value.split('/').next())
        .map(ToString::to_string);
    let last_commit = run_git_text(&context.repo_root, &["log", "-1", "--pretty=%h %s"]);
    let (ahead, behind) = upstream
        .as_ref()
        .and_then(|_| ahead_behind(&context.repo_root))
        .unwrap_or((0, 0));
    let files = status_files(context);

    GitSyncStatus {
        available: true,
        repo_found: true,
        repo_root: Some(context.repo_root.to_string_lossy().to_string()),
        branch,
        upstream,
        remote,
        ahead,
        behind,
        dirty: !files.is_empty(),
        files,
        last_commit,
        error: None,
    }
}

fn ahead_behind(repo_root: &Path) -> Option<(u32, u32)> {
    let output = run_git(
        repo_root,
        &["rev-list", "--left-right", "--count", "HEAD...@{u}"],
    )
    .ok()?;
    if output.status != 0 {
        return None;
    }
    let mut parts = output.stdout.split_whitespace();
    let ahead = parts.next()?.parse::<u32>().ok()?;
    let behind = parts.next()?.parse::<u32>().ok()?;
    Some((ahead, behind))
}

fn status_files(context: &GitContext) -> Vec<GitSyncFileStatus> {
    let mut args = vec!["status", "--porcelain", "--"];
    let journal_files = context
        .journal_files
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    args.extend(journal_files.iter().copied());

    run_git(&context.repo_root, &args)
        .ok()
        .filter(|output| output.status == 0)
        .map(|output| {
            output
                .stdout
                .lines()
                .filter_map(parse_status_line)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_status_line(line: &str) -> Option<GitSyncFileStatus> {
    if line.len() < 4 {
        return None;
    }
    let status = line.get(0..2)?.trim().to_string();
    let path = line.get(3..)?.trim().to_string();
    Some(GitSyncFileStatus { path, status })
}

fn run_git_text(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = run_git(cwd, args).ok()?;
    if output.status == 0 {
        let value = output.stdout.trim();
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    } else {
        None
    }
}

fn run_git_checked(cwd: &Path, args: &[&str]) -> Result<GitOutput, String> {
    let output = run_git(cwd, args)?;
    if output.status == 0 {
        Ok(output)
    } else {
        Err(command_error(&format!("git {}", args.join(" ")), &output))
    }
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<GitOutput, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| {
            to_error_string_with_details(
                "git_not_available",
                "Git is not available. Install Git or make it available in PATH.",
                error.to_string(),
            )
        })?;

    Ok(GitOutput {
        status: output.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

fn validate_commit_message(message: &str) -> Result<&str, String> {
    let message = message.trim();
    if message.is_empty() {
        return Err(to_error_string(
            "git_commit_message_required",
            "Enter a commit message.",
        ));
    }
    if message.contains('\n') || message.contains('\r') {
        return Err(to_error_string(
            "git_commit_message_single_line",
            "Commit message must be a single line.",
        ));
    }
    if message.chars().count() > 200 {
        return Err(to_error_string(
            "git_commit_message_too_long",
            "Commit message must be 200 characters or fewer.",
        ));
    }
    Ok(message)
}

fn command_error(command: &str, output: &GitOutput) -> String {
    let details = if output.stderr.trim().is_empty() {
        output.stdout.clone()
    } else {
        output.stderr.clone()
    };
    to_error_string_with_details("git_command_failed", command, details)
}
