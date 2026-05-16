//! Application backend for Ledgera.
//!
//! The Rust layer owns journal access, conservative transaction edits, settings
//! persistence, and integration with the official hledger CLI.

use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    journal_path: String,
    hledger_path: String,
    #[serde(default = "default_theme")]
    theme: String,
    #[serde(default)]
    power_user: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HledgerStatus {
    available: bool,
    version: String,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalSummary {
    path: String,
    transactions: Vec<JournalTransaction>,
    commodities: Vec<String>,
    dashboard: DashboardSummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardSummary {
    monthly_transactions: Vec<JournalTransaction>,
    scheduled_transactions: Vec<JournalTransaction>,
    active_accounts_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutocompleteSuggestions {
    codes: Vec<String>,
    descriptions: Vec<String>,
    accounts: Vec<String>,
    commodities: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalTransaction {
    id: String,
    source_file: String,
    date: String,
    status: String,
    code: String,
    description: String,
    postings: Vec<JournalPosting>,
    display: TransactionDisplay,
    raw: String,
    start_line: usize,
    end_line: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionDisplay {
    account: String,
    amount: String,
    kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalPosting {
    account: String,
    amount: String,
    commodity: String,
    comment: String,
    raw: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionInput {
    date: String,
    status: String,
    code: String,
    description: String,
    postings: Vec<PostingInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PostingInput {
    account: String,
    amount: String,
    #[serde(default)]
    commodity: String,
    #[serde(default)]
    comment: String,
}

#[derive(Debug, Clone)]
struct TransactionBlock {
    transaction: JournalTransaction,
}

#[derive(Debug, Clone)]
struct JournalFile {
    path: PathBuf,
    content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RoutingStrategy {
    Glob(Vec<String>),
    Flat(Vec<String>),
    Fallback,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            journal_path: String::new(),
            hledger_path: String::new(),
            theme: default_theme(),
            power_user: false,
        }
    }
}

fn default_theme() -> String {
    "system".to_string()
}

/// Returns persisted application settings from the platform config directory.
#[tauri::command]
fn get_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    read_settings(&app)
}

/// Persists application settings in the platform config directory.
#[tauri::command]
fn update_app_settings(app: AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    let path = settings_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let content = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())?;
    Ok(settings)
}

/// Checks whether the configured hledger executable can be invoked.
#[tauri::command]
fn check_hledger(app: AppHandle) -> Result<HledgerStatus, String> {
    let settings = read_settings(&app)?;
    let executable = hledger_executable(&settings);
    let output = Command::new(&executable).arg("--version").output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(HledgerStatus {
                available: true,
                version: version.clone(),
                message: version,
            })
        }
        Ok(output) => Ok(HledgerStatus {
            available: false,
            version: String::new(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        }),
        Err(error) => Ok(HledgerStatus {
            available: false,
            version: String::new(),
            message: error.to_string(),
        }),
    }
}

/// Reads distinct values that can be reused while editing transactions.
#[tauri::command]
fn get_autocomplete_suggestions(app: AppHandle) -> Result<AutocompleteSuggestions, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let transactions = load_transactions_from_journal(&journal_path)?;

    Ok(AutocompleteSuggestions {
        codes: unique_sorted(
            transactions
                .iter()
                .map(|transaction| transaction.code.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
        ),
        descriptions: unique_sorted(
            transactions
                .iter()
                .map(|transaction| transaction.description.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
        ),
        accounts: unique_sorted(
            transactions
                .iter()
                .flat_map(|transaction| transaction.postings.iter())
                .map(|posting| posting.account.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
        ),
        commodities: collect_commodities(&transactions),
    })
}

/// Reads the configured journal and returns parsed transaction blocks.
#[tauri::command]
fn list_transactions(app: AppHandle) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    read_journal_summary(&journal_path)
}

/// Appends a new transaction using the existing journal style where possible.
#[tauri::command]
fn create_transaction(app: AppHandle, input: TransactionInput) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    create_transaction_for_settings(&settings, &input)
}

/// Replaces an existing transaction block by id.
#[tauri::command]
fn update_transaction(
    app: AppHandle,
    id: String,
    input: TransactionInput,
) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    update_transaction_for_settings(&settings, &id, &input)
}

/// Removes an existing transaction block by id.
#[tauri::command]
fn delete_transaction(app: AppHandle, id: String) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    delete_transaction_for_settings(&settings, &id)
}

/// Starts the Tauri application and registers backend commands.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_settings,
            update_app_settings,
            check_hledger,
            get_autocomplete_suggestions,
            list_transactions,
            create_transaction,
            update_transaction,
            delete_transaction
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Builds the settings file path.
fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("settings.json"))
}

/// Reads settings, returning defaults if no settings file exists yet.
fn read_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

/// Resolves the hledger executable from settings or common macOS/user-shell locations.
fn hledger_executable(settings: &AppSettings) -> String {
    let configured = settings.hledger_path.trim();
    if !configured.is_empty() {
        return configured.to_string();
    }

    find_hledger_executable().unwrap_or_else(|| "hledger".to_string())
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
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
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

/// Resolves the configured journal path.
fn require_journal_path(settings: &AppSettings) -> Result<PathBuf, String> {
    if settings.journal_path.trim().is_empty() {
        return Err(
            "Configure a journal path in Settings before loading transactions.".to_string(),
        );
    }

    let path = PathBuf::from(settings.journal_path.trim());
    if !path.exists() {
        return Err(format!("Journal file does not exist: {}", path.display()));
    }
    Ok(path)
}

fn read_journal_summary(journal_path: &Path) -> Result<JournalSummary, String> {
    let transactions = load_transactions_from_journal(journal_path)?;
    let commodities = collect_commodities(&transactions);
    let dashboard = build_dashboard_summary(&transactions);

    Ok(JournalSummary {
        path: journal_path.to_string_lossy().to_string(),
        transactions,
        commodities,
        dashboard,
    })
}

fn load_transactions_from_journal(journal_path: &Path) -> Result<Vec<JournalTransaction>, String> {
    let files = load_journal_files(journal_path)?;
    Ok(files
        .iter()
        .flat_map(|file| parse_transactions(&file.content, &file.path))
        .collect())
}

fn load_journal_files(journal_path: &Path) -> Result<Vec<JournalFile>, String> {
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

    let content = fs::read_to_string(&canonical_path).map_err(|error| error.to_string())?;
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

fn parse_include_directive(line: &str) -> Option<String> {
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
            Err(format!(
                "Included journal file does not exist: {}",
                absolute_pattern.display()
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

fn create_transaction_for_settings(
    settings: &AppSettings,
    input: &TransactionInput,
) -> Result<JournalSummary, String> {
    let journal_path = require_journal_path(settings)?;
    append_transaction_routed(settings, &journal_path, input)?;
    read_journal_summary(&journal_path)
}

fn update_transaction_for_settings(
    settings: &AppSettings,
    id: &str,
    input: &TransactionInput,
) -> Result<JournalSummary, String> {
    let journal_path = require_journal_path(settings)?;
    let block = find_block(&journal_path, id)?;
    let source_path = PathBuf::from(&block.transaction.source_file);
    let replacement = format_transaction(input);
    mutate_existing_file(settings, &journal_path, &source_path, |content| {
        let lines = split_lines(content);
        Ok(replace_line_range(
            &lines,
            block.transaction.start_line,
            block.transaction.end_line,
            &replacement,
        ))
    })?;
    read_journal_summary(&journal_path)
}

fn delete_transaction_for_settings(
    settings: &AppSettings,
    id: &str,
) -> Result<JournalSummary, String> {
    let journal_path = require_journal_path(settings)?;
    let block = find_block(&journal_path, id)?;
    let source_path = PathBuf::from(&block.transaction.source_file);
    mutate_existing_file(settings, &journal_path, &source_path, |content| {
        let lines = split_lines(content);
        Ok(replace_line_range(
            &lines,
            block.transaction.start_line,
            block.transaction.end_line,
            "",
        ))
    })?;
    read_journal_summary(&journal_path)
}

fn mutate_existing_file<F>(
    settings: &AppSettings,
    main_journal: &Path,
    source_path: &Path,
    mutate: F,
) -> Result<(), String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    let original = fs::read_to_string(source_path).map_err(|error| error.to_string())?;
    let updated = mutate(&original)?;

    fs::write(source_path, &updated).map_err(|error| error.to_string())?;
    if let Err(error) = validate_journal(settings, main_journal) {
        fs::write(source_path, original).map_err(|rollback_error| rollback_error.to_string())?;
        return Err(error);
    }

    Ok(())
}

fn append_transaction_routed(
    settings: &AppSettings,
    main_journal: &Path,
    input: &TransactionInput,
) -> Result<(), String> {
    let content = fs::read_to_string(main_journal).map_err(|error| error.to_string())?;
    match detect_routing_strategy(&content) {
        RoutingStrategy::Fallback => {
            append_to_existing_file(settings, main_journal, main_journal, input)
        }
        RoutingStrategy::Flat(includes) => {
            let target_name = target_subjournal_name(input);
            let target_file = main_journal
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&target_name);
            if includes.contains(&target_name) || target_file.exists() {
                append_to_existing_file(settings, main_journal, &target_file, input)
            } else {
                append_to_new_flat_subjournal(
                    settings,
                    main_journal,
                    &target_file,
                    &target_name,
                    input,
                )
            }
        }
        RoutingStrategy::Glob(years) => {
            let (target_file, year) = glob_target_path(main_journal, input);
            if target_file.exists() {
                append_to_existing_file(settings, main_journal, &target_file, input)
            } else if years.contains(&year) {
                append_to_new_glob_subjournal(settings, main_journal, &target_file, input)
            } else {
                append_to_new_glob_year(settings, main_journal, &target_file, &year, input)
            }
        }
    }
}

fn append_to_existing_file(
    settings: &AppSettings,
    main_journal: &Path,
    target_file: &Path,
    input: &TransactionInput,
) -> Result<(), String> {
    mutate_existing_file(settings, main_journal, target_file, |content| {
        Ok(append_transaction_text(content, input))
    })
}

fn append_to_new_flat_subjournal(
    settings: &AppSettings,
    main_journal: &Path,
    target_file: &Path,
    target_name: &str,
    input: &TransactionInput,
) -> Result<(), String> {
    let original_main = fs::read_to_string(main_journal).map_err(|error| error.to_string())?;
    let updated_main = insert_include_sorted(&original_main, target_name);

    fs::write(main_journal, updated_main).map_err(|error| error.to_string())?;
    fs::write(target_file, format!("{}\n", format_transaction(input)))
        .map_err(|error| error.to_string())?;

    if let Err(error) = validate_journal(settings, main_journal) {
        let _ = fs::remove_file(target_file);
        fs::write(main_journal, original_main)
            .map_err(|rollback_error| rollback_error.to_string())?;
        return Err(error);
    }

    Ok(())
}

fn append_to_new_glob_subjournal(
    settings: &AppSettings,
    main_journal: &Path,
    target_file: &Path,
    input: &TransactionInput,
) -> Result<(), String> {
    if let Some(parent) = target_file.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(target_file, format!("{}\n", format_transaction(input)))
        .map_err(|error| error.to_string())?;

    if let Err(error) = validate_journal(settings, main_journal) {
        let _ = fs::remove_file(target_file);
        return Err(error);
    }

    Ok(())
}

fn append_to_new_glob_year(
    settings: &AppSettings,
    main_journal: &Path,
    target_file: &Path,
    year: &str,
    input: &TransactionInput,
) -> Result<(), String> {
    let original_main = fs::read_to_string(main_journal).map_err(|error| error.to_string())?;
    let year_dir = target_file
        .parent()
        .ok_or_else(|| "Unable to resolve target journal directory.".to_string())?;
    let year_dir_created = !year_dir.exists();
    let updated_main = insert_glob_include_sorted(&original_main, &format!("{}/*.journal", year));

    fs::write(main_journal, updated_main).map_err(|error| error.to_string())?;
    fs::create_dir_all(year_dir).map_err(|error| error.to_string())?;
    fs::write(target_file, format!("{}\n", format_transaction(input)))
        .map_err(|error| error.to_string())?;

    if let Err(error) = validate_journal(settings, main_journal) {
        let _ = fs::remove_file(target_file);
        if year_dir_created {
            let _ = fs::remove_dir(year_dir);
        }
        fs::write(main_journal, original_main)
            .map_err(|rollback_error| rollback_error.to_string())?;
        return Err(error);
    }

    Ok(())
}

fn append_transaction_text(content: &str, input: &TransactionInput) -> String {
    let mut updated = content.trim_end_matches(['\r', '\n']).to_string();
    if !updated.is_empty() {
        updated.push_str("\n\n");
    }
    updated.push_str(&format_transaction(input));
    updated.push('\n');
    updated
}

fn detect_routing_strategy(content: &str) -> RoutingStrategy {
    let glob_years = content
        .lines()
        .filter_map(parse_glob_include_year)
        .collect::<Vec<_>>();
    if !glob_years.is_empty() {
        return RoutingStrategy::Glob(glob_years);
    }

    let flat_files = content
        .lines()
        .filter_map(parse_flat_include_filename)
        .collect::<Vec<_>>();
    if !flat_files.is_empty() {
        return RoutingStrategy::Flat(flat_files);
    }

    RoutingStrategy::Fallback
}

fn parse_flat_include_filename(line: &str) -> Option<String> {
    let include = parse_include_directive(line)?;
    let file_name = Path::new(&include).file_name()?.to_str()?;
    if file_name.len() == "YYYY-MM.journal".len()
        && file_name.ends_with(".journal")
        && file_name
            .chars()
            .take(4)
            .all(|character| character.is_ascii_digit())
        && file_name.chars().nth(4) == Some('-')
        && file_name
            .chars()
            .skip(5)
            .take(2)
            .all(|character| character.is_ascii_digit())
    {
        Some(include)
    } else {
        None
    }
}

fn parse_glob_include_year(line: &str) -> Option<String> {
    let include = parse_include_directive(line)?;
    let (year, rest) = include.split_once('/')?;
    if year.len() == 4
        && year.chars().all(|character| character.is_ascii_digit())
        && rest == "*.journal"
    {
        Some(year.to_string())
    } else {
        None
    }
}

fn target_subjournal_name(input: &TransactionInput) -> String {
    format!("{}.journal", input.date.chars().take(7).collect::<String>())
}

fn glob_target_path(main_journal: &Path, input: &TransactionInput) -> (PathBuf, String) {
    let year = input.date.chars().take(4).collect::<String>();
    let month = input.date.chars().skip(5).take(2).collect::<String>();
    let target = main_journal
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&year)
        .join(format!("{}.journal", month));
    (target, year)
}

fn insert_include_sorted(content: &str, new_include: &str) -> String {
    insert_sorted_include(content, new_include, parse_flat_include_filename)
}

fn insert_glob_include_sorted(content: &str, new_include: &str) -> String {
    insert_sorted_include(content, new_include, |line| {
        parse_glob_include_year(line).map(|year| format!("{}/*.journal", year))
    })
}

fn insert_sorted_include<F>(content: &str, new_include: &str, parser: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let mut lines = content.lines().map(ToString::to_string).collect::<Vec<_>>();
    let include_positions = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| parser(line).map(|include| (index, include)))
        .collect::<Vec<_>>();
    let new_line = format!("include {}", new_include);

    if include_positions.is_empty() {
        lines.push(new_line);
    } else {
        let insert_index = include_positions
            .iter()
            .find(|(_, include)| new_include < include.as_str())
            .map(|(index, _)| *index)
            .unwrap_or_else(|| {
                include_positions
                    .last()
                    .map(|(index, _)| index + 1)
                    .unwrap_or(lines.len())
            });
        lines.insert(insert_index, new_line);
    }

    let mut updated = lines.join("\n");
    updated.push('\n');
    updated
}

/// Runs hledger check for the journal when hledger is available.
fn validate_journal(settings: &AppSettings, journal_path: &Path) -> Result<(), String> {
    let executable = hledger_executable(settings);
    let output = Command::new(&executable)
        .arg("-f")
        .arg(journal_path)
        .arg("check")
        .output();

    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        Err(error) => Err(format!("Unable to run hledger check: {}", error)),
    }
}

/// Parses transaction blocks without attempting to reinterpret ledger semantics.
fn parse_transactions(content: &str, source_path: &Path) -> Vec<JournalTransaction> {
    let lines = split_lines(content);
    let mut transactions = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        if !is_transaction_header(&lines[index]) {
            index += 1;
            continue;
        }

        let start_line = index + 1;
        let mut end_index = index + 1;
        while end_index < lines.len() && !is_transaction_header(&lines[end_index]) {
            end_index += 1;
        }

        let block_lines = &lines[index..end_index];
        let raw = block_lines.join("\n");
        if let Some(transaction) = parse_transaction_block(source_path, start_line, end_index, &raw)
        {
            transactions.push(transaction);
        }
        index = end_index;
    }

    transactions
}

/// Parses one transaction block.
fn parse_transaction_block(
    source_path: &Path,
    start_line: usize,
    end_line: usize,
    raw: &str,
) -> Option<JournalTransaction> {
    let mut lines = raw.lines();
    let header = lines.next()?.trim();
    let (date, rest) = split_first_token(header);
    let mut remaining = rest.trim_start();
    let mut status = String::new();
    let mut code = String::new();

    if remaining.starts_with('*') || remaining.starts_with('!') {
        status = remaining[..1].to_string();
        remaining = remaining[1..].trim_start();
    }

    if remaining.starts_with('(') {
        if let Some(end) = remaining.find(')') {
            code = remaining[..=end].to_string();
            remaining = remaining[end + 1..].trim_start();
        }
    }

    let postings = raw
        .lines()
        .skip(1)
        .filter_map(parse_posting)
        .collect::<Vec<_>>();

    let display = summarize_transaction(&postings);

    let source_file = source_path.to_string_lossy().to_string();
    Some(JournalTransaction {
        id: format!("{}:{}", source_file, start_line),
        source_file,
        date: date.to_string(),
        status,
        code,
        description: remaining.to_string(),
        postings,
        display,
        raw: raw.to_string(),
        start_line,
        end_line,
    })
}

/// Parses one posting line while preserving the raw text.
fn parse_posting(line: &str) -> Option<JournalPosting> {
    if line.trim().is_empty() || !line.starts_with(char::is_whitespace) {
        return None;
    }

    let trimmed = line.trim();
    if trimmed.starts_with(';') {
        return None;
    }

    let (posting_content, comment) = split_inline_comment(trimmed);
    let (account, amount) = split_posting_account_amount(posting_content);
    let (quantity, commodity) = parse_posting_amount(amount);
    Some(JournalPosting {
        account: account.trim().to_string(),
        amount: quantity,
        commodity,
        comment: comment.to_string(),
        raw: line.to_string(),
    })
}

/// Parses an hledger amount into quantity and commodity fields.
fn parse_posting_amount(amount: &str) -> (String, String) {
    let trimmed = amount.trim();
    if trimmed.is_empty() {
        return (String::new(), String::new());
    }

    let Some(number_start) = trimmed.find(|character: char| character.is_ascii_digit()) else {
        return (trimmed.to_string(), String::new());
    };

    let sign = if trimmed[..number_start].contains('-') {
        "-"
    } else {
        ""
    };
    let number_end = trimmed[number_start..]
        .char_indices()
        .find(|(_, character)| {
            !(character.is_ascii_digit() || *character == '.' || *character == ',')
        })
        .map(|(index, _)| number_start + index)
        .unwrap_or(trimmed.len());

    let quantity = format!("{}{}", sign, &trimmed[number_start..number_end]);
    let prefix_commodity = trimmed[..number_start].trim().trim_matches('-').trim();
    let suffix_commodity = trimmed[number_end..].trim();
    let commodity = if !prefix_commodity.is_empty() {
        prefix_commodity
    } else {
        suffix_commodity
    };

    (quantity, commodity.to_string())
}

/// Splits a posting into account and amount using hledger's common spacing convention.
fn split_posting_account_amount(value: &str) -> (&str, &str) {
    let mut whitespace_start = None;
    let mut whitespace_len = 0;

    for (index, character) in value.char_indices() {
        if character.is_whitespace() {
            if whitespace_start.is_none() {
                whitespace_start = Some(index);
            }
            whitespace_len += character.len_utf8();
            continue;
        }

        if let Some(start) = whitespace_start {
            if whitespace_len >= 2 {
                return (&value[..start], &value[index..]);
            }
        }
        whitespace_start = None;
        whitespace_len = 0;
    }

    (value, "")
}

/// Splits an inline hledger comment from a posting line.
fn split_inline_comment(value: &str) -> (&str, &str) {
    if let Some(index) = value.find(';') {
        (&value[..index], value[index + 1..].trim())
    } else {
        (value, "")
    }
}

/// Splits the first token from a string.
fn split_first_token(value: &str) -> (&str, &str) {
    if let Some(index) = value.find(char::is_whitespace) {
        (&value[..index], &value[index..])
    } else {
        (value, "")
    }
}

/// Returns true when a line appears to start a ledger transaction.
fn is_transaction_header(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
        return false;
    }

    trimmed
        .chars()
        .next()
        .map(|character| character.is_ascii_digit())
        .unwrap_or(false)
}

/// Finds a parsed transaction by id.
fn find_block(journal_path: &Path, id: &str) -> Result<TransactionBlock, String> {
    load_transactions_from_journal(journal_path)?
        .into_iter()
        .find(|transaction| transaction.id == id)
        .map(|transaction| TransactionBlock { transaction })
        .ok_or_else(|| format!("Transaction not found: {}", id))
}

/// Splits content into normalized lines for range replacement.
fn split_lines(content: &str) -> Vec<String> {
    content.lines().map(ToString::to_string).collect()
}

/// Replaces a one-based inclusive line range.
fn replace_line_range(
    lines: &[String],
    start_line: usize,
    end_line: usize,
    replacement: &str,
) -> String {
    let mut result = Vec::new();
    let start_index = start_line.saturating_sub(1);
    let end_index = end_line.min(lines.len());

    result.extend_from_slice(&lines[..start_index]);
    if !replacement.trim().is_empty() {
        result.extend(replacement.lines().map(ToString::to_string));
    }
    result.extend_from_slice(&lines[end_index..]);

    let mut content = result.join("\n");
    content.push('\n');
    content
}

/// Formats a transaction from structured form input.
fn format_transaction(input: &TransactionInput) -> String {
    let mut header = input.date.trim().to_string();
    if !input.status.trim().is_empty() {
        header.push(' ');
        header.push_str(input.status.trim());
    }
    if !input.code.trim().is_empty() {
        header.push(' ');
        header.push_str(input.code.trim());
    }
    if !input.description.trim().is_empty() {
        header.push(' ');
        header.push_str(input.description.trim());
    }

    let postings = input
        .postings
        .iter()
        .filter(|posting| !posting.account.trim().is_empty())
        .map(|posting| format_posting(posting))
        .collect::<Vec<_>>();

    std::iter::once(header)
        .chain(postings)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Normalizes a numeric quantity to two decimals when possible.
fn normalize_quantity(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let normalized = trimmed.replace(',', ".");
    match normalized.parse::<f64>() {
        Ok(value) => format!("{:.2}", value),
        Err(_) => trimmed.to_string(),
    }
}

/// Formats a quantity and commodity into hledger amount syntax.
fn format_posting_amount(amount: &str, commodity: &str) -> String {
    let (quantity, parsed_commodity) = parse_posting_amount(amount);
    let selected_commodity = if parsed_commodity.trim().is_empty() {
        commodity.trim()
    } else {
        parsed_commodity.trim()
    };
    let quantity = normalize_quantity(&quantity);

    if quantity.is_empty() {
        return String::new();
    }
    if selected_commodity.is_empty() {
        return quantity;
    }

    let sign = if quantity.starts_with('-') { "-" } else { "" };
    let absolute_quantity = quantity.trim_start_matches('-');
    if selected_commodity
        .chars()
        .all(|character| character.is_alphabetic())
    {
        format!("{}{} {}", sign, absolute_quantity, selected_commodity)
    } else {
        format!("{}{}{}", sign, selected_commodity, absolute_quantity)
    }
}

/// Formats a posting, including an optional hledger inline comment.
fn format_posting(posting: &PostingInput) -> String {
    let amount = format_posting_amount(&posting.amount, &posting.commodity);
    let mut line = if amount.trim().is_empty() {
        format!("    {}", posting.account.trim())
    } else {
        format!("    {:<40} {}", posting.account.trim(), amount)
    };

    if !posting.comment.trim().is_empty() {
        line.push_str("  ; ");
        line.push_str(posting.comment.trim().trim_start_matches(';').trim());
    }

    line
}

/// Builds the transaction display fields consumed by the frontend.
fn summarize_transaction(postings: &[JournalPosting]) -> TransactionDisplay {
    let postings_with_amounts = postings
        .iter()
        .filter(|posting| !posting.amount.trim().is_empty())
        .collect::<Vec<_>>();
    let balancing_amount = postings_with_amounts
        .first()
        .map(|posting| format_posting_amount(&posting.amount, &posting.commodity))
        .unwrap_or_default();

    if let Some(posting) = postings
        .iter()
        .find(|posting| posting.account.to_lowercase().starts_with("expenses"))
    {
        let amount = if posting.amount.trim().is_empty() {
            balancing_amount
        } else {
            format_posting_amount(&posting.amount, &posting.commodity)
        };
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: format_display_amount(&amount, "expense"),
            kind: "expense".to_string(),
        };
    }

    if let Some(posting) = postings
        .iter()
        .find(|posting| posting.account.to_lowercase().starts_with("income"))
    {
        let amount = if posting.amount.trim().is_empty() {
            balancing_amount
        } else {
            format_posting_amount(&posting.amount, &posting.commodity)
        };
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: format_display_amount(&amount, "income"),
            kind: "income".to_string(),
        };
    }

    if let Some(posting) = postings_with_amounts.iter().find(|posting| {
        posting.account.to_lowercase().starts_with("assets")
            && parse_amount_value(&posting.amount) > 0.0
    }) {
        let amount = format_posting_amount(&posting.amount, &posting.commodity);
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: format_display_amount(&amount, "income"),
            kind: "income".to_string(),
        };
    }

    if let Some(posting) = postings_with_amounts
        .first()
        .copied()
        .or_else(|| postings.first())
    {
        let amount = format_posting_amount(&posting.amount, &posting.commodity);
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: if amount.is_empty() {
                "—".to_string()
            } else {
                amount
            },
            kind: if posting.amount.trim().is_empty() {
                "unknown".to_string()
            } else {
                "transfer".to_string()
            },
        };
    }

    TransactionDisplay {
        account: "—".to_string(),
        amount: "—".to_string(),
        kind: "unknown".to_string(),
    }
}

/// Parses a numeric value from an amount quantity.
fn parse_amount_value(amount: &str) -> f64 {
    amount
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .unwrap_or_default()
}

/// Formats the display amount sign according to the inferred transaction kind.
fn format_display_amount(amount: &str, kind: &str) -> String {
    if amount.trim().is_empty() {
        return "—".to_string();
    }

    let normalized = amount.replace('-', "");
    match kind {
        "income" => format!("+{}", normalized),
        "expense" => format!("-{}", normalized),
        _ => amount.to_string(),
    }
}

/// Builds dashboard-specific transaction groups.
fn build_dashboard_summary(transactions: &[JournalTransaction]) -> DashboardSummary {
    let monthly_transactions = transactions
        .iter()
        .filter(|transaction| is_in_current_month_to_date(transaction))
        .cloned()
        .collect::<Vec<_>>();
    let scheduled_transactions = transactions
        .iter()
        .filter(|transaction| is_scheduled_this_month(transaction))
        .cloned()
        .collect::<Vec<_>>();
    let active_accounts_count = monthly_transactions
        .iter()
        .chain(scheduled_transactions.iter())
        .flat_map(|transaction| transaction.postings.iter())
        .map(|posting| posting.account.to_lowercase())
        .collect::<std::collections::HashSet<_>>()
        .len();

    DashboardSummary {
        monthly_transactions,
        scheduled_transactions,
        active_accounts_count,
    }
}

/// Returns whether a transaction belongs to the current month up to today.
fn is_in_current_month_to_date(transaction: &JournalTransaction) -> bool {
    let today = Local::now().date_naive();
    parse_journal_date(&transaction.date)
        .map(|date| date.year() == today.year() && date.month() == today.month() && date <= today)
        .unwrap_or(false)
}

/// Returns whether a transaction is scheduled later in the current month.
fn is_scheduled_this_month(transaction: &JournalTransaction) -> bool {
    let today = Local::now().date_naive();
    parse_journal_date(&transaction.date)
        .map(|date| date.year() == today.year() && date.month() == today.month() && date > today)
        .unwrap_or(false)
}

fn parse_journal_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

/// Sorts and removes duplicate values.
fn unique_sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort_by_key(|value| value.to_lowercase());
    values.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    values
}

/// Extracts commodity-like tokens from posting amounts for display.
fn collect_commodities(transactions: &[JournalTransaction]) -> Vec<String> {
    let mut commodities = transactions
        .iter()
        .flat_map(|transaction| transaction.postings.iter())
        .filter_map(|posting| {
            if posting.commodity.trim().is_empty() {
                None
            } else {
                Some(posting.commodity.as_str())
            }
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    commodities.sort();
    commodities.dedup();
    commodities
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("ledgera-{}-{}", name, nanos));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn settings_for(journal_path: &Path) -> AppSettings {
        AppSettings {
            journal_path: journal_path.to_string_lossy().to_string(),
            hledger_path: "true".to_string(),
            theme: default_theme(),
            power_user: false,
        }
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent dir should be created");
        }
        fs::write(path, content).expect("sample file should be written");
    }

    fn input(date: &str, description: &str) -> TransactionInput {
        TransactionInput {
            date: date.to_string(),
            status: "*".to_string(),
            code: String::new(),
            description: description.to_string(),
            postings: vec![
                PostingInput {
                    account: "expenses:test".to_string(),
                    amount: "10".to_string(),
                    commodity: "EUR".to_string(),
                    comment: String::new(),
                },
                PostingInput {
                    account: "assets:cash".to_string(),
                    amount: String::new(),
                    commodity: "EUR".to_string(),
                    comment: String::new(),
                },
            ],
        }
    }

    #[test]
    fn reads_transactions_from_flat_split_journal() {
        let dir = temp_dir("read-flat");
        let main = dir.join("main.journal");
        let may = dir.join("2026-05.journal");
        write_file(&main, "include 2026-05.journal\n\n2026-04-01 Main\n    assets:cash  1 EUR\n    equity:opening\n");
        write_file(
            &may,
            "2026-05-02 Split\n    expenses:office  10 EUR\n    assets:cash\n",
        );

        let summary = read_journal_summary(&main).expect("split journal should load");

        assert_eq!(summary.transactions.len(), 2);
        assert!(summary
            .transactions
            .iter()
            .any(|transaction| transaction.description == "Split"
                && transaction.source_file.ends_with("2026-05.journal")));
    }

    #[test]
    fn appends_to_existing_flat_subjournal_for_transaction_month() {
        let dir = temp_dir("append-flat-existing");
        let main = dir.join("main.journal");
        let may = dir.join("2026-05.journal");
        write_file(&main, "include 2026-05.journal\n");
        write_file(
            &may,
            "2026-05-02 Existing\n    expenses:office  10 EUR\n    assets:cash\n",
        );
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-05-20", "New split transaction"))
            .expect("transaction should be routed to existing split file");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        let may_content = fs::read_to_string(&may).expect("may journal should be readable");
        assert_eq!(main_content.matches("include 2026-05.journal").count(), 1);
        assert!(may_content.contains("2026-05-20 * New split transaction"));
    }

    #[test]
    fn creates_missing_flat_subjournal_and_sorted_include() {
        let dir = temp_dir("append-flat-new");
        let main = dir.join("main.journal");
        write_file(&main, "include 2026-04.journal\ninclude 2026-06.journal\n");
        write_file(&dir.join("2026-04.journal"), "");
        write_file(&dir.join("2026-06.journal"), "");
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-05-03", "Inserted month"))
            .expect("missing month journal should be created");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        let includes = main_content
            .lines()
            .filter(|line| line.starts_with("include"))
            .collect::<Vec<_>>();
        assert_eq!(
            includes,
            vec![
                "include 2026-04.journal",
                "include 2026-05.journal",
                "include 2026-06.journal",
            ]
        );
        assert!(fs::read_to_string(dir.join("2026-05.journal"))
            .expect("new split file should exist")
            .contains("2026-05-03 * Inserted month"));
    }

    #[test]
    fn creates_new_glob_year_and_month_file() {
        let dir = temp_dir("append-glob-new-year");
        let main = dir.join("main.journal");
        write_file(&main, "include 2025/*.journal\n");
        write_file(&dir.join("2025/12.journal"), "");
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-01-15", "New glob year"))
            .expect("new glob year should be created");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        assert!(main_content.contains("include 2026/*.journal"));
        assert!(fs::read_to_string(dir.join("2026/01.journal"))
            .expect("new glob month should exist")
            .contains("2026-01-15 * New glob year"));
    }

    #[test]
    fn update_and_delete_target_source_subjournal() {
        let dir = temp_dir("mutate-source");
        let main = dir.join("main.journal");
        let may = dir.join("2026-05.journal");
        write_file(&main, "include 2026-05.journal\n");
        write_file(
            &may,
            "2026-05-02 Original\n    expenses:office  10 EUR\n    assets:cash\n",
        );
        let settings = settings_for(&main);
        let summary = read_journal_summary(&main).expect("summary should load");
        let transaction_id = summary.transactions[0].id.clone();

        update_transaction_for_settings(
            &settings,
            &transaction_id,
            &input("2026-05-02", "Updated"),
        )
        .expect("source subjournal transaction should update");
        let updated_content = fs::read_to_string(&may).expect("may should be readable");
        assert!(updated_content.contains("2026-05-02 * Updated"));
        assert!(!updated_content.contains("Original"));

        let updated_summary = read_journal_summary(&main).expect("updated summary should load");
        delete_transaction_for_settings(&settings, &updated_summary.transactions[0].id)
            .expect("source subjournal transaction should delete");
        let deleted_content = fs::read_to_string(&may).expect("may should be readable");
        assert!(!deleted_content.contains("2026-05-02"));
    }

    #[test]
    fn configured_hledger_path_overrides_detection() {
        let settings = AppSettings {
            journal_path: String::new(),
            hledger_path: "/custom/bin/hledger".to_string(),
            theme: default_theme(),
            power_user: false,
        };

        assert_eq!(hledger_executable(&settings), "/custom/bin/hledger");
    }
}
